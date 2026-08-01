//! commands/write.rs - 写入类子命令:init/add/rm/update。
//!
//! 所有写操作都会增量更新 SQLite 索引,保持文件系统权威地位。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use regex::Regex;
use serde_json::{json, Value};

use crate::db::IndexDb;
use crate::error::KbError;
use crate::graph::{is_root_doc, norm_doc_arg};
use crate::graph_petgraph::KbGraph;
use crate::index::{format_tree, scan_files};
use crate::parser::{normalize_path, parse_frontmatter, parse_wikilinks};

const INDEX_TEMPLATE: &str = "---\nname: INDEX\nsummary: 知识库全局索引\ntags: [index]\nstatus: validated\n---\n```\n{tree}\n```\n";

/// markdown bullet 前缀正则:^[-*+]\s+(.*)$
static MD_BULLET_RE: OnceLock<Regex> = OnceLock::new();

fn md_bullet_re() -> &'static Regex {
    MD_BULLET_RE.get_or_init(|| {
        Regex::new(r"^[-*+]\s+(.*)$").expect("MD_BULLET_RE: 静态正则,编译期可验证,不会失败")
    })
}

/// 把 summary 平铺为 YAML 安全的单行字符串。
///
/// 多行:每行去掉 markdown bullet 前缀,非空行用 '; ' 连接。
fn flatten_summary(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    if !value.contains('\n') {
        let s = value.trim();
        if let Some(caps) = md_bullet_re().captures(s) {
            return caps
                .get(1)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
        }
        return s.to_string();
    }
    let mut cleaned: Vec<String> = Vec::new();
    for line in value.split('\n') {
        let s = line.trim();
        let s = if let Some(caps) = md_bullet_re().captures(s) {
            caps.get(1)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default()
        } else {
            s.to_string()
        };
        if !s.is_empty() {
            cleaned.push(s);
        }
    }
    cleaned.join("; ")
}

/// 生成 frontmatter 文本。
///
/// 手动构建 YAML,tags 使用行内数组格式 [t1,t2]。
fn yaml_scalar(s: &str) -> String {
    if s.is_empty() {
        return "''".to_string();
    }
    let needs_quote = s.starts_with('-')
        || s.starts_with(' ')
        || s.ends_with(' ')
        || s.contains(':')
        || s.contains('#')
        || s.contains('{')
        || s.contains('}')
        || s.contains('[')
        || s.contains(']')
        || s.contains('&')
        || s.contains('*')
        || s.contains('!')
        || s.contains('|')
        || s.contains('>')
        || s.contains('?')
        || s.contains('@')
        || s.contains('`')
        || s.contains('"')
        || s.contains('\'')
        || s.contains('%')
        || s.contains('\n')
        || matches!(s, "null" | "Null" | "NULL" | "~");
    if needs_quote {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        s.to_string()
    }
}

fn build_frontmatter(
    name: &str,
    summary: &str,
    category: &str,
    tags: &[&str],
    status: &str,
) -> Result<String, KbError> {
    let summary = flatten_summary(summary);
    let tags_str = if tags.is_empty() {
        "[]".to_string()
    } else {
        let items: Vec<String> = tags.iter().map(|t| yaml_scalar(t)).collect();
        format!("[{}]", items.join(", "))
    };
    let lines = vec![
        format!("name: {}", yaml_scalar(name)),
        format!("summary: {}", yaml_scalar(&summary)),
        format!("category: {}", yaml_scalar(category)),
        format!("tags: {}", tags_str),
        format!("status: {}", status),
    ];
    Ok(format!("---\n{}\n---", lines.join("\n")))
}

/// 组装完整文档:frontmatter + content,确保格式正确。
fn assemble_doc(frontmatter: &str, content: &str) -> String {
    let mut content = content.to_string();
    if !content.starts_with('\n') {
        content = format!("\n{}", content);
    }
    if !content.ends_with('\n') {
        content.push('\n');
    }
    format!("{}{}", frontmatter, content)
}

fn abs_path(kb_root_abs: &str, rel: &str) -> PathBuf {
    Path::new(kb_root_abs).join(rel.replace('/', std::path::MAIN_SEPARATOR_STR))
}

/// add_single 的返回结果(供 cmd_add 组装输出)。
#[derive(serde::Serialize)]
struct AddOutcome {
    doc: String,
    linked_from: String,
    relation: Option<String>,
    name: String,
    created_dirs: Vec<String>,
}

/// 重建 INDEX.md,返回(文件数, 可能的写入错误)。
fn rebuild_index_md(kb_root_abs: &str) -> (usize, Option<String>) {
    let index_path = Path::new(kb_root_abs).join("INDEX.md");
    let files = scan_files(kb_root_abs);
    let tree_str = format_tree(&files);
    let index_content = INDEX_TEMPLATE.replace("{tree}", &tree_str);
    match std::fs::write(&index_path, index_content) {
        Ok(()) => (files.len(), None),
        Err(e) => (files.len(), Some(format!("write INDEX.md: {}", e))),
    }
}

/// kb init <domain> --content <body> [...]
pub fn cmd_init(
    kb_root_abs: &str,
    domain: &str,
    summary: &str,
    category: Option<&str>,
    tags: Vec<String>,
    content: &str,
) -> Result<Value, KbError> {
    let domain = domain.trim().trim_matches('/').to_string();
    if domain.is_empty() {
        return Err(KbError::Other("domain name is empty".into()));
    }
    let mut created: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    // .knowledges/ 根目录
    if !Path::new(kb_root_abs).is_dir() {
        if let Err(e) = std::fs::create_dir_all(kb_root_abs) {
            return Err(KbError::Other(format!("creating kb root: {}", e)));
        }
        created.push("kb_root".to_string());
    }
    // 领域目录
    let domain_dir = abs_path(kb_root_abs, &domain);
    if domain_dir.exists() {
        warnings.push(format!("domain directory already exists: {}/", domain));
    } else {
        if let Err(e) = std::fs::create_dir_all(&domain_dir) {
            return Err(KbError::Other(format!("creating domain dir: {}", e)));
        }
        created.push(format!("{}/", domain));
    }
    // 根文档
    let root_doc_rel = format!("{}/{}.md", domain, domain);
    let root_doc_abs = domain_dir.join(format!("{}.md", domain));
    if root_doc_abs.exists() {
        return Err(KbError::Other(format!("root doc already exists: {}", root_doc_rel)));
    }
    let name = domain.clone();
    if summary.is_empty() {
        return Err(KbError::Other("summary is required (use --summary)".into()));
    }
    let category = category.unwrap_or(&domain).to_string();
    let tags = if tags.is_empty() {
        vec![domain.clone()]
    } else {
        tags
    };
    let frontmatter = build_frontmatter(
        &name,
        summary,
        &category,
        &tags.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        "pending",
    )?;
    let doc_content = assemble_doc(&frontmatter, content);
    if let Err(e) = std::fs::write(&root_doc_abs, doc_content) {
        return Err(KbError::Other(format!("writing root doc: {}", e)));
    }
    created.push(root_doc_rel);
    // INDEX.md
    let (total_documents, idx_err) = rebuild_index_md(kb_root_abs);
    if let Some(e) = idx_err {
        return Err(KbError::Other(e));
    }

    // 全量重建索引
    let index_updated = match IndexDb::open(kb_root_abs) {
        Ok(mut db) => match db.full_rebuild(kb_root_abs) {
            Ok(_) => true,
            Err(e) => {
                warnings.push(format!("index rebuild failed: {}", e));
                false
            }
        },
        Err(e) => {
            warnings.push(format!("index open failed: {}", e));
            false
        }
    };

    Ok(json!({
        "domain": domain,
        "kb_root": kb_root_abs,
        "name": name,
        "created": created,
        "index_updated": index_updated,
        "warnings": warnings,
        "total_documents": total_documents,
    }))
}

/// 单个文档创建逻辑(供 cmd_add 复用),不重建 INDEX。
///
/// 返回 Ok(result_json) 或 Err(error_msg)。
fn add_single(
    kb_root_abs: &str,
    doc_path: &str,
    link_from: &str,
    content: &str,
    summary: &str,
    relation: Option<&str>,
    category: Option<&str>,
    tags: Option<&[String]>,
) -> Result<AddOutcome, String> {
    let mut doc_path = normalize_path(doc_path, ".knowledges");
    if !doc_path.ends_with(".md") {
        doc_path.push_str(".md");
    }
    let parent = normalize_path(link_from, ".knowledges");
    // 检查新文档不存在
    let new_abs = abs_path(kb_root_abs, &doc_path);
    if new_abs.exists() {
        return Err(format!("document already exists: {}", doc_path));
    }
    // 检查父文档存在
    let parent_abs = abs_path(kb_root_abs, &parent);
    if !parent_abs.exists() {
        return Err(format!("parent document not found: {}", parent));
    }
    // 创建目录
    let mut created_dirs: Vec<String> = Vec::new();
    if let Some(parent_dir) = new_abs.parent() {
        if !parent_dir.is_dir() {
            if let Err(e) = std::fs::create_dir_all(parent_dir) {
                return Err(format!("creating dir: {}", e));
            }
            let dir_name = doc_path
                .rsplit_once('/')
                .map(|(d, _)| format!("{}/", d))
                .unwrap_or_default();
            created_dirs.push(dir_name);
        }
    }
    // name 从文件名推导
    let basename = doc_path.rsplit('/').next().unwrap_or(&doc_path);
    let name = basename.trim_end_matches(".md").to_string();
    if summary.is_empty() {
        return Err("summary is required".to_string());
    }
    let category = category.unwrap_or("").to_string();
    let tags: Vec<String> = tags.map(|t| t.iter().map(|s| s.to_string()).collect()).unwrap_or_default();
    let frontmatter = build_frontmatter(
        &name,
        summary,
        &category,
        &tags.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        "pending",
    )
    .map_err(|e| e.to_string())?;
    let doc_content = assemble_doc(&frontmatter, content);
    if let Err(e) = std::fs::write(&new_abs, doc_content) {
        return Err(format!("writing doc: {}", e));
    }
    // 在父文档追加 wiki-link
    let mut parent_content = match std::fs::read_to_string(&parent_abs) {
        Ok(t) => t,
        Err(e) => return Err(format!("reading parent: {}", e)),
    };
    let link_line = if let Some(relation) = relation {
        if !relation.is_empty() {
            format!("- [[`.knowledges/{}`|{}]]", doc_path, relation)
        } else {
            format!("- [[`.knowledges/{}`]]", doc_path)
        }
    } else {
        format!("- [[`.knowledges/{}`]]", doc_path)
    };
    if !parent_content.ends_with('\n') {
        parent_content.push('\n');
    }
    parent_content.push_str(&link_line);
    parent_content.push('\n');
    if let Err(e) = std::fs::write(&parent_abs, parent_content) {
        return Err(format!("writing parent: {}", e));
    }
    Ok(AddOutcome {
        doc: doc_path,
        linked_from: parent,
        relation: relation.map(|s| s.to_string()),
        name,
        created_dirs,
    })
}

/// kb add <doc-path> --link-from <parent> --content <body> [...]
pub fn cmd_add(
    db: &mut IndexDb,
    kb_root_abs: &str,
    doc_path: &str,
    link_from: &str,
    relation: Option<&str>,
    summary: &str,
    category: Option<&str>,
    tags: Vec<String>,
    content: &str,
) -> Result<Value, KbError> {
    let o = match add_single(
        kb_root_abs, doc_path, link_from, content, summary, relation, category, Some(&tags),
    ) {
        Ok(o) => o,
        Err(e) => return Err(KbError::Other(e)),
    };
    let mut index_warnings: Vec<String> = Vec::new();
    let doc_ok = match db.upsert_doc(kb_root_abs, &o.doc) {
        Ok(()) => true,
        Err(e) => {
            index_warnings.push(format!("upsert {}: {}", o.doc, e));
            false
        }
    };
    let parent_ok = match db.upsert_doc(kb_root_abs, &o.linked_from) {
        Ok(()) => true,
        Err(e) => {
            index_warnings.push(format!("upsert {}: {}", o.linked_from, e));
            false
        }
    };

    let (total, idx_err) = rebuild_index_md(kb_root_abs);
    if let Some(e) = idx_err {
        index_warnings.push(e);
    }
    Ok(json!({
        "doc": o.doc,
        "linked_from": o.linked_from,
        "relation": o.relation,
        "name": o.name,
        "created_dirs": o.created_dirs,
        "created": true,
        "index_updated": doc_ok && parent_ok,
        "total_documents": total,
        "index_warnings": index_warnings,
    }))
}

/// kb rm <doc> - 只报告影响,不删文件;从索引中移除。
pub fn cmd_rm(db: &mut IndexDb, kb_root_abs: &str, doc: &str) -> Result<Value, KbError> {
    let doc = norm_doc_arg(doc, ".knowledges");
    let abs = abs_path(kb_root_abs, &doc);
    if !abs.exists() {
        return Err(KbError::Other(format!("document not found: {}", doc)));
    }

    let graph = match KbGraph::from_index(db) {
        Ok(g) => g,
        Err(e) => return Err(KbError::Other(format!("graph build failed: {}", e))),
    };

    let outlinks = match db.outlinks(&doc) {
        Ok(o) => o,
        Err(e) => return Err(KbError::Other(format!("outlinks query failed: {}", e))),
    };
    let inlinks = match db.inlinks(&doc) {
        Ok(i) => i,
        Err(e) => return Err(KbError::Other(format!("inlinks query failed: {}", e))),
    };

    let doc_paths: Vec<String> = match db.all_docs_meta() {
        Ok(metas) => metas.into_iter().map(|m| m.path).collect(),
        Err(e) => return Err(KbError::Other(format!("docs meta query failed: {}", e))),
    };
    let doc_paths_set: std::collections::HashSet<String> =
        doc_paths.iter().cloned().collect();

    let inlinks_data: Vec<Value> = inlinks
        .iter()
        .map(|(s, l)| json!({"doc": s, "label": l}))
        .collect();
    let outlinks_data: Vec<Value> = outlinks
        .iter()
        .map(|(t, l)| json!({"doc": t, "label": l, "exists": doc_paths_set.contains(t)}))
        .collect();

    // 孤儿风险
    let mut orphan_risk: Vec<String> = Vec::new();
    for (tgt, _) in &outlinks {
        if !doc_paths_set.contains(tgt) {
            continue;
        }
        let tgt_inlinks = match db.inlinks(tgt) {
            Ok(i) => i,
            Err(_) => continue,
        };
        let has_other_source = tgt_inlinks
            .iter()
            .any(|(s, _)| s != &doc && doc_paths_set.contains(s));
        if !has_other_source {
            orphan_risk.push(tgt.clone());
        }
    }

    // 连通性检查
    let mut reachability: BTreeMap<String, String> = BTreeMap::new();
    if !orphan_risk.is_empty() {
        let roots: Vec<String> = doc_paths
            .iter()
            .filter(|p| is_root_doc(p) && *p != &doc)
            .cloned()
            .collect();
        for tgt in &orphan_risk {
            let mut reachable = false;
            for root in &roots {
                let reached = graph.reachable_set(root, None, true);
                if reached.contains(tgt) {
                    reachable = true;
                    break;
                }
            }
            reachability.insert(
                tgt.clone(),
                if reachable {
                    "reachable".to_string()
                } else {
                    "unreachable".to_string()
                },
            );
        }
    }

    // 从索引中移除(文件系统保留)
    if let Err(e) = db.remove_doc(&doc) {
        return Err(KbError::Other(format!("remove from index failed: {}", e)));
    }

    Ok(json!({
        "doc": doc,
        "abs_path": abs.to_string_lossy(),
        "deleted": false,
        "inlinks": inlinks_data,
        "outlinks": outlinks_data,
        "orphan_risk": orphan_risk,
        "reachability": reachability,
        "hint": format!("to actually delete, run: rm {} (inlinks 字段列出指向本文档的引用,删除后它们会变成 dangling)", abs.display()),
    }))
}

/// kb update <doc> [--content | --append] [--summary] [--name] [--tags] [--add-link --to]
pub fn cmd_update(
    db: &mut IndexDb,
    kb_root_abs: &str,
    doc: &str,
    content: Option<&str>,
    append: Option<&str>,
    summary: Option<&str>,
    name: Option<&str>,
    tags: Vec<String>,
    add_link: bool,
    to: Option<&str>,
    label: Option<&str>,
) -> Result<Value, KbError> {
    let doc = norm_doc_arg(doc, ".knowledges");
    let abs = abs_path(kb_root_abs, &doc);
    let text = match std::fs::read_to_string(&abs) {
        Ok(t) => t,
        Err(_) => return Err(KbError::Other(format!("document not found: {}", doc))),
    };
    let (fm, body, _has_fm) = parse_frontmatter(&text);
    let mut name_val = fm.name;
    let mut summary_val = fm.summary;
    let category_val = fm.category;
    let mut tags_val = fm.tags;

    let mut changes: Vec<String> = Vec::new();
    // --name 更新
    if let Some(n) = name {
        name_val = n.to_string();
        changes.push("name".to_string());
    }
    // --summary 更新
    if let Some(s) = summary {
        summary_val = s.to_string();
        changes.push("summary".to_string());
    }
    // --tags 更新
    if !tags.is_empty() {
        tags_val = tags.clone();
        changes.push("tags".to_string());
    }
    // --content 替换正文 / --append 追加
    let mut new_body = body;
    if let Some(c) = content {
        new_body = c.to_string();
        if !new_body.ends_with('\n') {
            new_body.push('\n');
        }
        changes.push("content".to_string());
    } else if let Some(a) = append {
        if !new_body.ends_with('\n') {
            new_body.push('\n');
        }
        new_body.push_str(a);
        if !new_body.ends_with('\n') {
            new_body.push('\n');
        }
        changes.push("append".to_string());
    }
    // --add-link 追加 wiki-link
    if add_link {
        let to = match to {
            Some(t) => t,
            None => return Err(KbError::Other("--add-link requires --to <doc>".into())),
        };
        let target = normalize_path(to, ".knowledges");
        let link_line = if let Some(label) = label {
            if !label.is_empty() {
                format!("- [[`.knowledges/{}`|{}]]", target, label)
            } else {
                format!("- [[`.knowledges/{}`]]", target)
            }
        } else {
            format!("- [[`.knowledges/{}`]]", target)
        };
        if !new_body.ends_with('\n') {
            new_body.push('\n');
        }
        new_body.push_str(&link_line);
        new_body.push('\n');
        changes.push(format!("add-link:{}", target));
    }

    if changes.is_empty() {
        return Err(KbError::Other("no update specified (use --content/--append/--summary/--name/--add-link)".into()));
    }

    // 重建 frontmatter + body
    let frontmatter = build_frontmatter(
        &name_val,
        &summary_val,
        &category_val,
        &tags_val.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        "pending",
    )?;
    let new_text = assemble_doc(&frontmatter, &new_body);
    if let Err(e) = std::fs::write(&abs, new_text) {
        return Err(KbError::Other(format!("writing {}: {}", doc, e)));
    }

    // 增量更新索引
    if let Err(e) = db.upsert_doc(kb_root_abs, &doc) {
        return Err(KbError::Other(format!("index update failed: {}", e)));
    }

    Ok(json!({
        "doc": doc,
        "changes": changes,
        "updated": true,
    }))
}


/// 审核通过:将文档 status 改为 validated 并更新索引。
/// 这是给 GUI(Tauri) 调用的核心 API,CLI 不暴露。
/// review 后会重新 upsert 所有指向该文档的父文档,恢复入链。
pub fn cmd_review(
    db: &mut IndexDb,
    kb_root_abs: &str,
    doc: &str,
) -> Result<Value, KbError> {
    let doc = norm_doc_arg(doc, ".knowledges");
    let abs = abs_path(kb_root_abs, &doc);
    let text = std::fs::read_to_string(&abs)
        .map_err(|e| KbError::Other(format!("reading {}: {}", doc, e)))?;
    let (fm, body, has_fm) = parse_frontmatter(&text);
    if !has_fm {
        return Err(KbError::Other(format!("document has no frontmatter: {}", doc)));
    }
    let already_validated = fm.status == "validated";
    let frontmatter = build_frontmatter(
        &fm.name,
        &fm.summary,
        &fm.category,
        &fm.tags.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        "validated",
    )?;
    let new_text = assemble_doc(&frontmatter, &body);
    std::fs::write(&abs, new_text)
        .map_err(|e| KbError::Other(format!("writing {}: {}", doc, e)))?;
    db.upsert_doc(kb_root_abs, &doc)?;
    // Re-upsert parent docs that link to this doc to restore incoming links
    // that were deleted while the doc was pending.
    // Skip when already validated: links are already in place from the prior review.
    let mut index_warnings: Vec<String> = Vec::new();
    if !already_validated {
        let doc_with_md = if doc.ends_with(".md") { doc.clone() } else { format!("{}.md", doc) };
        let files = scan_files(kb_root_abs);
        for file in &files {
            if file == &doc { continue; }
            let file_abs = abs_path(kb_root_abs, file);
            let file_text = match std::fs::read_to_string(&file_abs) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let links = parse_wikilinks(&file_text, ".knowledges");
            let links_to_doc = links.iter().any(|(target, _)| {
                let t = if target.ends_with(".md") { target.clone() } else { format!("{}.md", target) };
                t == doc_with_md
            });
            if links_to_doc {
                if let Err(e) = db.upsert_doc(kb_root_abs, file) {
                    index_warnings.push(format!("re-upsert {}: {}", file, e));
                }
            }
        }
    }
    Ok(json!({
        "doc": doc,
        "status": "validated",
        "already_validated": already_validated,
        "index_warnings": index_warnings,
    }))
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_frontmatter;
    use std::io::Write;
    fn setup_kb() -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().to_string();
        (dir, root)
    }

    fn setup_kb_with_docs() -> (tempfile::TempDir, String, IndexDb) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().to_string();
        let result = cmd_init(
            &root,
            "zoloz",
            "root summary",
            None,
            vec![],
            "root content"
        );
        assert!(result.is_ok());
        let mut db = IndexDb::open(&root).unwrap();
        cmd_review(&mut db, &root, "zoloz/zoloz.md").unwrap();
        (dir, root, db)
    }

    fn write_doc(root: &str, rel: &str, content: &str) {
        let abs = Path::new(root).join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut f = std::fs::File::create(&abs).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    fn read_doc(root: &str, rel: &str) -> String {
        let abs = Path::new(root).join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        std::fs::read_to_string(&abs).unwrap()
    }

    // ===== cmd_init =====

    #[test]
    fn test_init_creates_structure() {
        let (_dir, root) = setup_kb();
        let result = cmd_init(
            &root,
            "zoloz",
            "zoloz summary",
            None,
            vec!["tag1".to_string()],
            "root body"
        );
        assert!(result.is_ok());
        let v = result.unwrap();
        assert!(v.get("command").is_none(), "core 返回值不应含 command 字段");

        // domain 目录存在
        assert!(Path::new(&root).join("zoloz").is_dir());
        // root doc 存在
        let root_doc = Path::new(&root).join("zoloz").join("zoloz.md");
        assert!(root_doc.exists());
        // INDEX.md 存在
        assert!(Path::new(&root).join("INDEX.md").exists());

        // frontmatter 含 name/summary/tags
        let text = read_doc(&root, "zoloz/zoloz.md");
        let (fm, _, has_fm) = parse_frontmatter(&text);
        assert!(has_fm);
        assert_eq!(fm.name, "zoloz");
        assert_eq!(fm.summary, "zoloz summary");
        assert!(fm.tags.contains(&"tag1".to_string()));

        // full_rebuild 后索引有 1 文档
        let mut db = IndexDb::open(&root).unwrap();
        cmd_review(&mut db, &root, "zoloz/zoloz.md").unwrap();
        let stats = db.full_rebuild(&root).unwrap();
        assert_eq!(stats.indexed, 1);
        let docs = db.all_docs_meta().unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].path, "zoloz/zoloz.md");
    }

    #[test]
    fn test_init_root_doc_exists_error() {
        let (_dir, root) = setup_kb();
        // 预先创建 root doc
        write_doc(&root, "zoloz/zoloz.md", "existing");
        let result = cmd_init(&root, "zoloz", "s", None, vec![], "c");
        assert!(result.is_err());
    }

    #[test]
    fn test_init_empty_domain() {
        let (_dir, root) = setup_kb();
        let result = cmd_init(&root, "", "s", None, vec![], "c");
        assert!(result.is_err());
    }

    #[test]
    fn test_init_default_name_tags() {
        let (_dir, root) = setup_kb();
        let result = cmd_init(&root, "zoloz", "s", None, vec![], "c");
        assert!(result.is_ok());
        let mut db = IndexDb::open(&root).unwrap();
        cmd_review(&mut db, &root, "zoloz/zoloz.md").unwrap();
        let text = read_doc(&root, "zoloz/zoloz.md");
        let (fm, _, _) = parse_frontmatter(&text);
        // 不传 name/tags 时默认用 domain
        assert_eq!(fm.name, "zoloz");
        assert!(fm.tags.contains(&"zoloz".to_string()));
    }

    // ===== cmd_add =====

    #[test]
    fn test_add_creates_doc_and_link() {
        let (_dir, root, mut db) = setup_kb_with_docs();
        let before_count = db.all_docs_meta().unwrap().len();
        assert_eq!(before_count, 1);

        let result = cmd_add(
            &mut db,
            &root,
            "zoloz/sub.md",
            "zoloz/zoloz.md",
            None,
            "sub summary",
            None,
            vec!["tag1".to_string()],
            "sub content"
        );
        cmd_review(&mut db, &root, "zoloz/sub.md").unwrap();
        db.upsert_doc(&root, "zoloz/zoloz.md").unwrap();
        assert!(result.is_ok());

        // 新文档存在
        assert!(Path::new(&root).join("zoloz").join("sub.md").exists());

        // 父文档末尾含 wiki-link
        let parent_text = read_doc(&root, "zoloz/zoloz.md");
        assert!(parent_text.contains("[[`.knowledges/zoloz/sub.md`]]"));

        // db.outlinks(parent) 含新文档
        let out = db.outlinks("zoloz/zoloz.md").unwrap();
        assert!(out.iter().any(|(t, _)| t == "zoloz/sub.md"));

        // all_docs_meta 数量+1
        let after_count = db.all_docs_meta().unwrap().len();
        assert_eq!(after_count, before_count + 1);
    }

    #[test]
    fn test_add_with_label() {
        let (_dir, root, mut db) = setup_kb_with_docs();
        let result = cmd_add(
            &mut db,
            &root,
            "zoloz/sub.md",
            "zoloz/zoloz.md",
            Some("关系"),
            "sub summary",
            None,
            vec!["tag1".to_string()],
            "sub content"
        );
        cmd_review(&mut db, &root, "zoloz/sub.md").unwrap();
        db.upsert_doc(&root, "zoloz/zoloz.md").unwrap();
        assert!(result.is_ok());

        // 父文档 link 行含 |关系 标签
        let parent_text = read_doc(&root, "zoloz/zoloz.md");
        assert!(parent_text.contains("|关系]]"));

        // db.outlinks 返回 Some(label)
        let out = db.outlinks("zoloz/zoloz.md").unwrap();
        let found = out.iter().find(|(t, _)| t == "zoloz/sub.md");
        assert!(found.is_some());
        assert_eq!(found.unwrap().1.as_deref(), Some("关系"));
    }

    #[test]
    fn test_add_doc_exists_error() {
        let (_dir, root, mut db) = setup_kb_with_docs();
        // 先创建文档
        cmd_add(
            &mut db,
            &root,
            "zoloz/sub.md",
            "zoloz/zoloz.md",
            None,
            "sub summary",
            None,
            vec!["tag1".to_string()],
            "content"
        ).unwrap();
        cmd_review(&mut db, &root, "zoloz/sub.md").unwrap();
        // 再次添加同文档
        let result = cmd_add(
            &mut db,
            &root,
            "zoloz/sub.md",
            "zoloz/zoloz.md",
            None,
            "sub summary",
            None,
            vec!["tag1".to_string()],
            "content"
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_add_parent_not_found_error() {
        let (_dir, root, mut db) = setup_kb_with_docs();
        let result = cmd_add(
            &mut db,
            &root,
            "zoloz/sub.md",
            "zoloz/nonexistent.md",
            None,
            "sub summary",
            None,
            vec!["tag1".to_string()],
            "content"
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_add_auto_md_ext() {
        let (_dir, root, mut db) = setup_kb_with_docs();
        // doc_path 不带 .md
        let result = cmd_add(
            &mut db,
            &root,
            "zoloz/noext",
            "zoloz/zoloz.md",
            None,
            "s",
            None,
            vec!["tag1".to_string()],
            "c"
        );
        cmd_review(&mut db, &root, "zoloz/noext.md").unwrap();
        db.upsert_doc(&root, "zoloz/zoloz.md").unwrap();
        assert!(result.is_ok());
        // 自动补齐 .md
        assert!(Path::new(&root).join("zoloz").join("noext.md").exists());
    }

    // ===== cmd_update =====    // ===== cmd_update =====

    #[test]
    fn test_update_content() {
        let (_dir, root, mut db) = setup_kb_with_docs();
        cmd_add(
            &mut db,
            &root,
            "zoloz/a.md",
            "zoloz/zoloz.md",
            None,
            "a summary",
            None,
            vec!["tag1".to_string()],
            "original body"
        ).unwrap();
        cmd_review(&mut db, &root, "zoloz/a.md").unwrap();

        let result = cmd_update(
            &mut db,
            &root,
            "zoloz/a.md",
            Some("completely new content"),
            None,
            None,
            None,
            vec![],
            false,
            None,
            None,
        );
        assert!(result.is_ok());
        cmd_review(&mut db, &root, "zoloz/a.md").unwrap();

        // 正文被替换
        let text = read_doc(&root, "zoloz/a.md");
        let (_fm, body, _) = parse_frontmatter(&text);
        assert!(body.contains("completely new content"));
        assert!(!body.contains("original body"));

        // db.get_doc_bodies 返回新内容
        let bodies = db.get_doc_bodies(&["zoloz/a.md"]);
        assert_eq!(
            bodies.get("zoloz/a.md").map(|s| s.as_str()),
            Some("completely new content\n")
        );
    }

    #[test]
    fn test_update_append() {
        let (_dir, root, mut db) = setup_kb_with_docs();
        cmd_add(
            &mut db,
            &root,
            "zoloz/a.md",
            "zoloz/zoloz.md",
            None,
            "a summary",
            None,
            vec!["tag1".to_string()],
            "original body"
        ).unwrap();
        cmd_review(&mut db, &root, "zoloz/a.md").unwrap();

        let result = cmd_update(
            &mut db,
            &root,
            "zoloz/a.md",
            None,
            Some(" appended text"),
            None,
            None,
            vec![],
            false,
            None,
            None,
        );
        assert!(result.is_ok());

        // 正文末尾追加,原内容保留
        let text = read_doc(&root, "zoloz/a.md");
        let (_fm, body, _) = parse_frontmatter(&text);
        assert!(body.contains("original body"));
        assert!(body.contains("appended text"));
    }

    #[test]
    fn test_update_summary_name_tags() {
        let (_dir, root, mut db) = setup_kb_with_docs();
        cmd_add(
            &mut db,
            &root,
            "zoloz/a.md",
            "zoloz/zoloz.md",
            Some("oldname"),
            "old summary",
            None,
            vec!["tag1".to_string()],
            "body"
        ).unwrap();
        cmd_review(&mut db, &root, "zoloz/a.md").unwrap();

        let result = cmd_update(
            &mut db,
            &root,
            "zoloz/a.md",
            None,
            None,
            Some("new summary"),
            Some("newname"),
            vec!["newtag".to_string()],
            false,
            None,
            None,
        );
        assert!(result.is_ok());
        cmd_review(&mut db, &root, "zoloz/a.md").unwrap();

        // all_docs_meta 反映新值
        let docs = db.all_docs_meta().unwrap();
        let doc = docs.iter().find(|d| d.path == "zoloz/a.md").unwrap();
        assert_eq!(doc.name, "newname");
        assert_eq!(doc.summary, "new summary");
        assert!(doc.tags.contains(&"newtag".to_string()));
    }

    #[test]
    fn test_update_add_link() {
        let (_dir, root, mut db) = setup_kb_with_docs();
        cmd_add(
            &mut db,
            &root,
            "zoloz/a.md",
            "zoloz/zoloz.md",
            None,
            "a summary",
            None,
            vec!["tag1".to_string()],
            "body"
        ).unwrap();
        cmd_review(&mut db, &root, "zoloz/a.md").unwrap();

        // a.md 原本无 outlinks
        let before = db.outlinks("zoloz/a.md").unwrap();
        assert!(before.is_empty());

        let result = cmd_update(
            &mut db,
            &root,
            "zoloz/a.md",
            None,
            None,
            None,
            None,
            vec![],
            true,
            Some("zoloz/target.md"),
            None,
        );
        assert!(result.is_ok());
        cmd_review(&mut db, &root, "zoloz/a.md").unwrap();
        // target.md 需要存在且 validated 才能在索引中保留 link
        write_doc(&root, "zoloz/target.md", "---\nname: target\nsummary: target\ntags: []\nstatus: validated\n---\ntarget body");
        db.upsert_doc(&root, "zoloz/target.md").unwrap();
        // a.md 的 outlinks 需要重新 upsert 才能看到 target.md
        db.upsert_doc(&root, "zoloz/a.md").unwrap();

        // outlinks 数量+1
        let after = db.outlinks("zoloz/a.md").unwrap();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].0, "zoloz/target.md");

        // 正文末尾新增 wiki-link
        let text = read_doc(&root, "zoloz/a.md");
        assert!(text.contains("[[`.knowledges/zoloz/target.md`]]"));
    }

    #[test]
    fn test_update_no_args_error() {
        let (_dir, root, mut db) = setup_kb_with_docs();
        cmd_add(
            &mut db,
            &root,
            "zoloz/a.md",
            "zoloz/zoloz.md",
            None,
            "a summary",
            None,
            vec!["tag1".to_string()],
            "body"
        ).unwrap();
        cmd_review(&mut db, &root, "zoloz/a.md").unwrap();
        let result = cmd_update(
            &mut db,
            &root,
            "zoloz/a.md",
            None,
            None,
            None,
            None,
            vec![],
            false,
            None,
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_update_doc_not_found_error() {
        let (_dir, root, mut db) = setup_kb_with_docs();
        let result = cmd_update(
            &mut db,
            &root,
            "zoloz/nonexistent.md",
            Some("c"),
            None,
            None,
            None,
            vec![],
            false,
            None,
            None,
        );
        assert!(result.is_err());
    }

    // ===== cmd_rm =====

    #[test]
    fn test_rm_reports_and_keeps_file() {
        let (_dir, root, mut db) = setup_kb_with_docs();
        cmd_add(
            &mut db,
            &root,
            "zoloz/a.md",
            "zoloz/zoloz.md",
            None,
            "a summary",
            None,
            vec!["tag1".to_string()],
            "a body"
        ).unwrap();
        cmd_review(&mut db, &root, "zoloz/a.md").unwrap();
        let doc_abs = Path::new(&root).join("zoloz").join("a.md");

        let result = cmd_rm(&mut db, &root, "zoloz/a.md");
        assert!(result.is_ok());

        // 文件仍存在(deleted=false)
        assert!(doc_abs.exists());

        // 索引中已移除
        let docs = db.all_docs_meta().unwrap();
        assert!(!docs.iter().any(|d| d.path == "zoloz/a.md"));

        // outlinks 为空
        let out = db.outlinks("zoloz/a.md").unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn test_rm_orphan_risk() {
        let (_dir, root, mut db) = setup_kb_with_docs();
        // root -> a -> b
        cmd_add(
            &mut db,
            &root,
            "zoloz/a.md",
            "zoloz/zoloz.md",
            None,
            "a summary",
            None,
            vec!["tag1".to_string()],
            "a body"
        ).unwrap();
        cmd_review(&mut db, &root, "zoloz/a.md").unwrap();
        db.upsert_doc(&root, "zoloz/zoloz.md").unwrap();
        cmd_add(
            &mut db,
            &root,
            "zoloz/b.md",
            "zoloz/a.md",
            None,
            "b summary",
            None,
            vec!["tag1".to_string()],
            "b body"
        ).unwrap();
        cmd_review(&mut db, &root, "zoloz/b.md").unwrap();
        db.upsert_doc(&root, "zoloz/a.md").unwrap();
        // 删除前交叉验证:b 的唯一入链源是 a
        let b_inlinks = db.inlinks("zoloz/b.md").unwrap();
        assert!(!b_inlinks.is_empty());
        assert!(b_inlinks.iter().all(|(s, _)| s == "zoloz/a.md"));

        let result = cmd_rm(&mut db, &root, "zoloz/a.md");
        assert!(result.is_ok());

        // 删除后 b 的入链被清空(确认 orphan risk 成立)
        let b_inlinks_after = db.inlinks("zoloz/b.md").unwrap();
        assert!(b_inlinks_after.is_empty());

        // b 仍在索引中(只是变成孤儿)
        let docs = db.all_docs_meta().unwrap();
        assert!(docs.iter().any(|d| d.path == "zoloz/b.md"));
    }

    #[test]
    fn test_rm_not_found_error() {
        let (_dir, root, mut db) = setup_kb_with_docs();
        let result = cmd_rm(&mut db, &root, "zoloz/nonexistent.md");
        assert!(result.is_err());
    }

    #[test]
    fn test_build_frontmatter_special_chars() {
        // name 含 YAML 危险字符(: # 等)不应破坏 frontmatter
        let fm = build_frontmatter("a: b", "summary", "cat", &["t1"], "pending").unwrap();
        let (parsed, _body, has_fm) =
            parse_frontmatter(&format!("{}\nbody", fm));
        assert!(has_fm);
        assert_eq!(parsed.name, "a: b");
        assert_eq!(parsed.summary, "summary");
        assert_eq!(parsed.category, "cat");
        assert_eq!(parsed.tags, vec!["t1".to_string()]);
    }
}
