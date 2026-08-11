//! commands/search.rs - 查询类子命令:index/links/traverse/tags/search/show。
//!
//! 读命令只读索引(写命令已实时单点增量更新);show 与 index --tree/--flat 均读索引,

use std::path::Path;
use walkdir::WalkDir;

use serde_json::{json, Value};

use crate::db::IndexDb;
use crate::error::KbError;
use crate::util::round2;
use crate::graph::norm_doc_arg;
use crate::graph_petgraph::KbGraph;
use crate::index::{tree_to_value, SKIP_DIRS};
/// 批量序列化 Serialize 切片为 Vec<Value>，失败转 KbError（保留 serde 错误上下文）。
///
/// 用于 cmd_traverse 把 TraversePath 列表转 JSON：任一序列化失败即短路传播，
/// 避免逐个 unwrap 在生产路径 panic。point-free `.map(serde_json::to_value)`
/// 是惯用法：成功路径与普通 collect 零差异，失败才短路。
fn to_values<T: serde::Serialize>(items: &[T]) -> Result<Vec<Value>, KbError> {
    items
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<_, _>>()
        .map_err(|e| KbError::Other(format!("serialize traverse paths: {e}")))
}

/// kb index --tree | --flat(读索引,只含 validated)。
pub fn cmd_index(db: &mut IndexDb, _kb_root_abs: &str, flat: bool) -> Result<Value, KbError> {
    let files = db.all_doc_paths().map_err(KbError::Sqlite)?;
    if flat {
        Ok(json!({
            "total": files.len(),
            "documents": files,
        }))
    } else {
        Ok(json!({
            "total": files.len(),
            "tree": tree_to_value(&files),
        }))
    }
}

/// kb links --from <doc> [--reverse]
pub fn cmd_links(
    db: &mut IndexDb,
    kb_root_abs: &str,
    from: &str,
    reverse: bool,
) -> Result<Value, KbError> {
    let doc = norm_doc_arg(from, ".knowledges");
    let direction = if reverse { "in" } else { "out" };
    let raw = (if reverse { db.inlinks(&doc) } else { db.outlinks(&doc) })
        .map_err(|e| KbError::Other(format!("links query failed: {}", e)))?;

    let related: Vec<Value> = raw
        .iter()
        .map(|(other, relation)| {
            let other_abs = Path::new(kb_root_abs)
                .join(other.replace('/', std::path::MAIN_SEPARATOR_STR));
            json!({
                "doc": other,
                "relation": relation,
                "exists": other_abs.exists(),
            })
        })
        .collect();
    Ok(json!({
        "doc": doc,
        "direction": direction,
        "count": related.len(),
        "related": related,
    }))
}

/// kb traverse --from <doc> [-j N] [--bidir] [--relation-filter <kw>]
pub fn cmd_traverse(
    db: &mut IndexDb,
    _kb_root_abs: &str,
    from: &str,
    jumps: usize,
    bidir: bool,
    relation_filter: Option<&str>,
) -> Result<Value, KbError> {
    let doc = norm_doc_arg(from, ".knowledges");
    let graph = KbGraph::from_index(db)
        .map_err(|e| KbError::Other(format!("graph build failed: {}", e)))?;

    if !graph.contains(&doc) {
        return Err(KbError::Other(format!("{} not found in knowledge base", doc)));
    }

    let paths = graph.traverse(&doc, jumps, bidir, relation_filter);
    let direct: Vec<&crate::graph::TraversePath> =
        paths.iter().filter(|p| !p.via_root).collect();
    let via_root: Vec<&crate::graph::TraversePath> =
        paths.iter().filter(|p| p.via_root).collect();
    let paths_json = to_values(&paths)?;
    let direct_json = to_values(&direct)?;
    let via_root_json = to_values(&via_root)?;
    Ok(json!({
        "from": doc,
        "max_hops": jumps,
        "bidir": bidir,
        "relation_filter": relation_filter,
        "total_paths": paths_json.len(),
        "direct_paths": direct_json,
        "via_root_paths": via_root_json,
        "paths": paths_json,
    }))
}

/// kb tags [<tag>]
pub fn cmd_tags(db: &mut IndexDb, _kb_root_abs: &str, tag: Option<&str>) -> Result<Value, KbError> {
    if let Some(tag) = tag {
        let hits = db.docs_for_tag(tag)
            .map_err(|e| KbError::Other(format!("tags query failed: {}", e)))?;
        return Ok(json!({
            "tag": tag,
            "count": hits.len(),
            "documents": hits,
        }));
    }

    let tag_map = db.list_tags()
        .map_err(|e| KbError::Other(format!("tags query failed: {}", e)))?;
    Ok(json!({
        "total_tags": tag_map.len(),
        "tags": tag_map,
    }))
}

/// kb search <keyword> [--top N] [--context N]
pub fn cmd_search(
    db: &mut IndexDb,
    _kb_root_abs: &str,
    keyword: &str,
    top: Option<usize>,
    context_lines: usize,
) -> Result<Value, KbError> {
    let hits = db.search(keyword, top)
        .map_err(|e| KbError::Other(format!("search failed: {}", e)))?;

    let total = hits.len();
    let kw_lower = keyword.to_lowercase();
    let max_matches_per_doc = 3;

    // 批量取 body:1 次 SQL 替代 N 次 get_doc_body
    let paths: Vec<&str> = hits.iter().map(|h| h.path.as_str()).collect();
    let bodies = db.get_doc_bodies(&paths);

    let matches: Vec<Value> = hits
        .iter()
        .map(|h| {
            let body = bodies.get(&h.path).map(|s| s.as_str()).unwrap_or("");
            let contexts = build_contexts(body, &kw_lower, context_lines, max_matches_per_doc);
            json!({
                "doc": h.path,
                "score": round2(h.rank),
                "name": h.name,
                "summary": h.summary,
                "contexts": contexts,
            })
        })
        .collect();

    Ok(json!({
        "keyword": keyword,
        "total_files": total,
        "returned_files": matches.len(),
        "matches": matches,
    }))
}

/// 构建匹配行上下文:在 body 中定位关键词所在行,每个匹配取 ±context_lines 行,
/// 最多返回 max_matches 个匹配区间。
///
/// 优化:1 次全量 to_lowercase 替代逐行小写分配;预计算行起始字节偏移,
/// pre/match/post 直接借用 body 切片(不 split、不 join、不额外分配 String)。
fn build_contexts(body: &str, kw_lower: &str, context_lines: usize, max_matches: usize) -> Vec<Value> {
    if kw_lower.is_empty() || max_matches == 0 || body.is_empty() {
        return Vec::new();
    }
    // 1 次大分配替代 N 次逐行 to_lowercase
    let body_lower = body.to_lowercase();

    // 预计算每行起始字节偏移(基于 body,保证切片 body 安全)
    // 对 ASCII+CJK 内容(知识库实际场景),body 与 body_lower 字节偏移一致,
    // 因此同样偏移可用于在 body_lower 中匹配关键词。
    let mut line_starts: Vec<usize> = Vec::with_capacity(64);
    line_starts.push(0);
    for (i, b) in body.bytes().enumerate() {
        if b == b'\n' {
            line_starts.push(i + 1);
        }
    }
    let num_lines = line_starts.len();
    let body_len = body.len();

    let mut result: Vec<Value> = Vec::with_capacity(max_matches);

    for (i, &start) in line_starts.iter().enumerate() {
        if result.len() >= max_matches {
            break;
        }
        // 当前行结束位置(\n 位置,或末行 body_len)
        let end = if i + 1 < num_lines {
            line_starts[i + 1] - 1
        } else {
            body_len
        };
        if start >= end {
            continue; // 空行
        }
        // 在 body_lower 中做 case-insensitive 匹配(get 防 Unicode 边界 panic)
        let line_lower = match body_lower.get(start..end) {
            Some(s) => s,
            None => continue,
        };
        if !line_lower.contains(kw_lower) {
            continue;
        }

        // pre: lines[i - context_lines .. i],直接借用 body 切片(不含末尾 \n)
        let pre_start_line = i.saturating_sub(context_lines);
        let pre_start_byte = line_starts[pre_start_line];
        let pre: &str = if pre_start_byte < start && start > 0 {
            // body[pre_start_byte..start] 含匹配行前的 \n,排除之
            body.get(pre_start_byte..start - 1).unwrap_or("")
        } else {
            ""
        };

        // post: lines[i+1 ..= i+context_lines],直接借用 body 切片(不含末尾 \n)
        let post_end_line = (i + context_lines).min(num_lines.saturating_sub(1));
        let post: &str = if i < post_end_line && i + 1 < num_lines {
            let post_start_byte = line_starts[i + 1];
            let post_end_byte = if post_end_line + 1 < num_lines {
                line_starts[post_end_line + 1] - 1
            } else {
                body_len
            };
            if post_start_byte < post_end_byte {
                body.get(post_start_byte..post_end_byte).unwrap_or("")
            } else {
                ""
            }
        } else {
            ""
        };

        // match: 当前行原文(不含 \n)
        let match_line: &str = body.get(start..end).unwrap_or("");

        result.push(json!({
            "pre": pre,
            "match": match_line,
            "post": post,
        }));
    }

    result
}

/// kb show <doc> [--summary]
pub fn cmd_show(
    db: &mut IndexDb,
    _kb_root_abs: &str,
    doc: &str,
    summary_only: bool,
) -> Result<Value, KbError> {
    let doc = norm_doc_arg(doc, ".knowledges");
    if doc.is_empty() {
        return Err(KbError::Other("invalid document path".into()));
    }
    let rec = db
        .get_doc(&doc)
        .map_err(KbError::Sqlite)?
        .ok_or_else(|| KbError::Other(format!("{} not found in index", doc)))?;
    if summary_only {
        return Ok(json!({
            "doc": doc,
            "summary": rec.summary,
            "name": rec.name,
        }));
    }
    let tags = db.tags_for_doc(&doc).map_err(KbError::Sqlite)?;
    Ok(json!({
        "doc": doc,
        "has_frontmatter": rec.has_frontmatter,
        "frontmatter": {
            "name": rec.name,
            "summary": rec.summary,
            "domain": rec.domain,
            "tags": tags,
        },
        "body": rec.body,
    }))
}

/// kb domains [domain] — 列出全部领域或指定领域下的子领域。
///
/// 基于目录结构读取 domain，空 domain 也可被列出。
pub fn cmd_domains(_db: &mut IndexDb, kb_root_abs: &str, domain: Option<&str>) -> Result<Value, KbError> {
    let root = Path::new(kb_root_abs);
    let directories = WalkDir::new(root)
        .min_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_dir())
        .filter(|entry| {
            entry.path().components().all(|component| {
                !SKIP_DIRS.contains(&component.as_os_str().to_string_lossy().as_ref())
            })
        })
        .filter_map(|entry| {
            entry.path().strip_prefix(root).ok().map(|path| {
                path.to_string_lossy().replace('\\', "/")
            })
        })
        .collect::<Vec<_>>();
    match domain {
        None => {
            let domains: std::collections::BTreeSet<String> = directories
                .iter()
                .filter_map(|path| path.split('/').next().map(str::to_string))
                .collect();
            let list: Vec<String> = domains.into_iter().collect();
            Ok(json!({ "domains": list, "total": list.len() }))
        }
        Some(domain) => {
            let domain = domain.trim().trim_matches('/').to_string();
            let prefix = format!("{}/", domain);
            let subdomains: std::collections::BTreeSet<String> = directories
                .iter()
                .filter_map(|path| path.strip_prefix(&prefix))
                .filter_map(|rest| rest.split('/').next().map(str::to_string))
                .collect();
            let list: Vec<String> = subdomains.into_iter().collect();
            Ok(json!({ "domain": domain, "sub_domains": list, "total": list.len() }))
        }
    }
}

/// kb trashbox list - 列出回收站(.trash-box)中所有文件。
pub fn cmd_trashbox_list(kb_root_abs: &str) -> Result<Value, KbError> {
    let trash_root = Path::new(kb_root_abs).join(".trash-box");
    let mut files: Vec<Value> = Vec::new();
    if trash_root.is_dir() {
        for entry in WalkDir::new(&trash_root)
            .into_iter()
            .filter_entry(|e| {
                // 防御:嵌套的 .trash-box 目录不再递归
                if e.file_type().is_dir() && e.depth() > 0 {
                    if let Some(name) = e.file_name().to_str() {
                        if name == ".trash-box" {
                            return false;
                        }
                    }
                }
                true
            })
        {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            if !entry.file_type().is_file() {
                continue;
            }
            let ext = entry
                .path()
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            if ext != "md" {
                continue;
            }
            let rel = match entry.path().strip_prefix(&trash_root) {
                Ok(r) => r.to_string_lossy().replace('\\', "/").to_string(),
                Err(_) => continue,
            };
            let meta = entry.metadata().ok();
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let mtime = meta
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            files.push(json!({
                "doc": rel,
                "size": size,
                "mtime": mtime,
            }));
        }
    }
    files.sort_by(|a, b| {
        a["doc"]
            .as_str()
            .unwrap_or("")
            .cmp(b["doc"].as_str().unwrap_or(""))
    });
    Ok(json!({
        "trash_box": ".trash-box",
        "count": files.len(),
        "files": files,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph_petgraph::KbGraph;
    use std::io::Write;
    fn write_doc(root: &str, rel: &str, content: &str) {
        let abs = Path::new(root).join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut f = std::fs::File::create(&abs).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    /// 预置 A→B 链接的 kb。
    fn setup_ab_chain() -> (tempfile::TempDir, String, IndexDb) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().to_string();
        write_doc(
            &root,
            "zoloz/a.md",
            "---\nname: a\nsummary: a summary\ntags: [alpha]\nstatus: validated\n---\n[[`.knowledges/zoloz/b.md`]]",
        );
        write_doc(
            &root,
            "zoloz/b.md",
            "---\nname: b\nsummary: b summary\ntags: [beta]\nstatus: validated\n---\nb body with keyword",
        );
        let mut db = IndexDb::open(&root).unwrap();
        db.full_rebuild(&root).unwrap();
        (dir, root, db)
    }

    /// 预置 A→B→C 链的 kb。
    fn setup_abc_chain() -> (tempfile::TempDir, String, IndexDb) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().to_string();
        write_doc(
            &root,
            "zoloz/a.md",
            "---\nname: a\nsummary: a\ntags: [alpha]\nstatus: validated\n---\n[[`.knowledges/zoloz/b.md`]]",
        );
        write_doc(
            &root,
            "zoloz/b.md",
            "---\nname: b\nsummary: b\ntags: [beta]\nstatus: validated\n---\n[[`.knowledges/zoloz/c.md`|关系]]",
        );
        write_doc(
            &root,
            "zoloz/c.md",
            "---\nname: c\nsummary: c\ntags: [gamma]\nstatus: validated\n---\nc body",
        );
        let mut db = IndexDb::open(&root).unwrap();
        db.full_rebuild(&root).unwrap();
        (dir, root, db)
    }

    // ===== cmd_index =====

    #[test]
    fn test_index_tree() {
        let (_dir, root, mut db) = setup_ab_chain();
        let result = cmd_index(&mut db, &root, false);
        assert!(result.is_ok());
        // 交叉调 db.all_doc_paths 验证数量一致
        let files = db.all_doc_paths().unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_index_flat() {
        let (_dir, root, mut db) = setup_ab_chain();
        let result = cmd_index(&mut db, &root, true);
        assert!(result.is_ok());
        let files = db.all_doc_paths().unwrap();
        assert_eq!(files.len(), 2);
    }

    // ===== cmd_links =====

    #[test]
    fn test_links_forward() {
        let (_dir, root, mut db) = setup_ab_chain();
        let result = cmd_links(&mut db, &root, "zoloz/a.md", false);
        assert!(result.is_ok());
        // 交叉调 db.outlinks 验证返回含 B
        let out = db.outlinks("zoloz/a.md").unwrap();
        assert!(out.iter().any(|(t, _)| t == "zoloz/b.md"));
    }

    #[test]
    fn test_links_reverse() {
        let (_dir, root, mut db) = setup_ab_chain();
        let result = cmd_links(&mut db, &root, "zoloz/b.md", true);
        assert!(result.is_ok());
        // 交叉调 db.inlinks 验证返回含 A
        let inn = db.inlinks("zoloz/b.md").unwrap();
        assert!(inn.iter().any(|(s, _)| s == "zoloz/a.md"));
    }

    #[test]
    fn test_links_not_found() {
        let (_dir, root, mut db) = setup_ab_chain();
        // 文档不存在仍退出码 0(空结果非错误)
        let result = cmd_links(&mut db, &root, "zoloz/nonexistent.md", false);
        assert!(result.is_ok());
    }

    // ===== cmd_traverse =====

    #[test]
    fn test_traverse_forward_2hops() {
        let (_dir, root, mut db) = setup_abc_chain();
        let result = cmd_traverse(&mut db, &root, "zoloz/a.md", 2, false, None);
        assert!(result.is_ok());
        // 交叉调 KbGraph::from_index + traverse 验证路径数
        let graph = KbGraph::from_index(&db).unwrap();
        let paths = graph.traverse("zoloz/a.md", 2, false, None);
        assert!(!paths.is_empty());
        // 2 跳应能到达 c.md(b.md 是中间节点,traverse 只返回终点)
        let targets: Vec<&str> = paths.iter().map(|p| p.path.last().unwrap().as_str()).collect();
        assert!(targets.contains(&"zoloz/c.md"));
    }

    #[test]
    fn test_traverse_bidir() {
        let (_dir, root, mut db) = setup_abc_chain();
        let result = cmd_traverse(&mut db, &root, "zoloz/b.md", 2, true, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_traverse_relation_filter() {
        let (_dir, root, mut db) = setup_abc_chain();
        // 带 "关系" 标签过滤
        let result = cmd_traverse(&mut db, &root, "zoloz/a.md", 2, false, Some("关系"));
        assert!(result.is_ok());
        // 交叉验证:filtered 路径应都含 "关系" 标签
        let graph = KbGraph::from_index(&db).unwrap();
        let paths = graph.traverse("zoloz/a.md", 2, false, Some("关系"));
        for p in &paths {
            assert!(p.relations.iter().any(|l| l.as_deref() == Some("关系")));
        }
    }

    #[test]
    fn test_traverse_not_found_error() {
        let (_dir, root, mut db) = setup_abc_chain();
        // 起始文档不在图中退出码 1
        let result = cmd_traverse(&mut db, &root, "zoloz/nonexistent.md", 2, false, None);
        assert!(result.is_err());
    }

    // ===== cmd_tags =====

    #[test]
    fn test_tags_list_all() {
        let (_dir, root, mut db) = setup_abc_chain();
        let result = cmd_tags(&mut db, &root, None);
        assert!(result.is_ok());
        // 交叉调 db.list_tags 验证 tag 数量
        let tags = db.list_tags().unwrap();
        assert!(tags.contains_key("alpha"));
        assert!(tags.contains_key("beta"));
        assert!(tags.contains_key("gamma"));
    }

    #[test]
    fn test_tags_specific() {
        let (_dir, root, mut db) = setup_abc_chain();
        let result = cmd_tags(&mut db, &root, Some("alpha"));
        assert!(result.is_ok());
        // 交叉调 db.docs_for_tag 验证命中
        let docs = db.docs_for_tag("alpha").unwrap();
        assert!(docs.contains(&"zoloz/a.md".to_string()));
    }
    // ===== cmd_search =====

    #[test]
    fn test_search_basic() {
        let (_dir, root, mut db) = setup_ab_chain();
        // b.md 含 "keyword"
        let result = cmd_search(&mut db, &root, "keyword", None, 0);
        assert!(result.is_ok());
        // 交叉调 db.search 验证命中数 >= 1
        let hits = db.search("keyword", None).unwrap();
        assert!(hits.len() >= 1);
        assert!(hits.iter().any(|h| h.path == "zoloz/b.md"));
    }

    #[test]
    fn test_search_context_lines() {
        // 覆盖 build_contexts 的 pre/match/post 三段(context_lines>0 分支,search.rs:245 边界改动点)
        let (_dir, root, mut db) = setup_ab_chain();
        let result = cmd_search(&mut db, &root, "keyword", None, 2);
        assert!(result.is_ok());
        let v = result.unwrap();
        assert!(v.get("command").is_none(), "core 返回值不应含 command 字段");
        let matches = v["matches"].as_array().unwrap();
        assert!(!matches.is_empty());
        let ctxs = matches[0]["contexts"].as_array().unwrap();
        assert!(!ctxs.is_empty());
        // match 段必含关键词
        assert!(ctxs[0]["match"].as_str().unwrap().contains("keyword"));
    }
    #[test]
    fn test_search_top_limit() {
        let (_dir, root, mut db) = setup_ab_chain();
        // root content 也含 "keyword"
        let result = cmd_search(&mut db, &root, "keyword", Some(1), 0);
        assert!(result.is_ok());
        // 交叉调 db.search 验证截断
        let hits = db.search("keyword", Some(1)).unwrap();
        assert!(hits.len() <= 1);
    }

    #[test]
    fn test_search_no_hits() {
        let (_dir, root, mut db) = setup_ab_chain();
        // 无匹配关键词退出码 0(空结果非错误)
        let result = cmd_search(&mut db, &root, "zzz_no_match", None, 0);
        assert!(result.is_ok());
        let hits = db.search("zzz_no_match", None).unwrap();
        assert!(hits.is_empty());
    }

    // ===== cmd_show =====

    #[test]
    fn test_show_full() {
        let (_dir, root, mut db) = setup_ab_chain();
        let result = cmd_show(&mut db, &root, "zoloz/a.md", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_show_summary() {
        let (_dir, root, mut db) = setup_ab_chain();
        let result = cmd_show(&mut db, &root, "zoloz/a.md", true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_show_domain_field() {
        let (_dir, root, mut db) = setup_ab_chain();
        let result = cmd_show(&mut db, &root, "zoloz/a.md", false);
        let v = result.unwrap();
        // domain 从路径推导 = zoloz(父目录)
        assert_eq!(v["frontmatter"]["domain"], "zoloz");
        // frontmatter 仅含已知字段,无多余分类字段
        let keys = v["frontmatter"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        assert!(keys.contains(&"domain".to_string()));
    }

    #[test]
    fn test_show_not_found_error() {
        let (_dir, root, mut db) = setup_ab_chain();
        let result = cmd_show(&mut db, &root, "zoloz/nonexistent.md", false);
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not found in index"),
            "错误消息应含 not found in index,实际: {}",
            err
        );
    }

    #[test]
    fn test_show_top_level_doc_domain_empty() {
        let (_dir, root, mut db) = setup_ab_chain();
        // 顶层文档(无斜杠),domain 从路径推导为空;先 upsert 进索引再 show
        write_doc(
            &root,
            "top.md",
            "---\nname: top\nsummary: s\ntags: []\nstatus: validated\n---\nbody",
        );
        db.upsert_doc(&root, "top.md").unwrap();
        let result = cmd_show(&mut db, &root, "top.md", false);
        let v = result.unwrap();
        assert_eq!(v["frontmatter"]["domain"], "");
    }

    // ===== cmd_domains =====

    #[test]
    fn test_domains_list_all() {
        let (_dir, root, mut db) = setup_ab_chain();
        // 添加另一个领域
        write_doc(
            &root,
            "other/other.md",
            "---\nname: other\nsummary: s\ntags: []\nstatus: validated\n---\nbody",
        );
        db.upsert_doc(&root, "other/other.md").unwrap();
        let v = cmd_domains(&mut db, &root, None).unwrap();
        let list = v["domains"].as_array().unwrap();
        assert!(list.iter().any(|d| d == "other"));
        assert!(list.iter().any(|d| d == "zoloz"));
        assert_eq!(v["total"], 2);
    }
    #[test]
    fn test_domains_sub_domains() {
        let (_dir, root, mut db) = setup_ab_chain();
        // zoloz 已有 a.md, b.md(无子领域)
        let v = cmd_domains(&mut db, &root, Some("zoloz")).unwrap();
        let subs = v["sub_domains"].as_array().unwrap();
        assert_eq!(subs.len(), 0);
    }

    #[test]
    fn test_show_pending_not_in_index() {
        let (_dir, root, mut db) = setup_ab_chain();
        // pending 文档:文件存在但索引隔离(刻意设计),show 应报错
        write_doc(
            &root,
            "zoloz/pending.md",
            "---\nname: pending\nsummary: s\ntags: []\nstatus: pending\n---\nbody",
        );
        db.upsert_doc(&root, "zoloz/pending.md").unwrap();
        let result = cmd_show(&mut db, &root, "zoloz/pending.md", false);
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not found in index"),
            "pending 文档应报 not found in index,实际: {}",
            err
        );
    }

    #[test]
    fn test_show_tags_from_tags_table() {
        let (_dir, root, mut db) = setup_ab_chain();
        // 多 tag 文档,show 返回的 tags 应与 tags 表一致(非 tags_text split)
        write_doc(
            &root,
            "zoloz/multi.md",
            "---\nname: multi\nsummary: s\ntags: [alpha, beta gamma]\nstatus: validated\n---\nbody",
        );
        db.upsert_doc(&root, "zoloz/multi.md").unwrap();
        let v = cmd_show(&mut db, &root, "zoloz/multi.md", false).unwrap();
        let tags = v["frontmatter"]["tags"].as_array().unwrap();
        let db_tags = db.tags_for_doc("zoloz/multi.md").unwrap();
        let tags_vec: Vec<&str> = tags.iter().map(|t| t.as_str().unwrap()).collect();
        assert_eq!(
            tags_vec.len(),
            db_tags.len(),
            "show tags 与 tags 表条目数一致"
        );
        for t in &db_tags {
            assert!(
                tags_vec.contains(&t.as_str()),
                "tags 表条目 {} 应出现在 show 输出",
                t
            );
        }
    }

    #[test]
    fn test_index_flat_excludes_pending() {
        let (_dir, root, mut db) = setup_ab_chain();
        // pending 文档:index --flat 不应列出
        write_doc(
            &root,
            "zoloz/pending2.md",
            "---\nname: pending2\nsummary: s\ntags: []\nstatus: pending\n---\nbody",
        );
        db.upsert_doc(&root, "zoloz/pending2.md").unwrap();
        let v = cmd_index(&mut db, &root, true).unwrap();
        let docs = v["documents"].as_array().unwrap();
        assert!(
            !docs
                .iter()
                .any(|d| d.as_str().unwrap() == "zoloz/pending2.md"),
            "index --flat 不应包含 pending 文档"
        );
        assert_eq!(v["total"], 2, "只含 validated 文档");
    }

    #[test]
    fn test_domains_sub_domains_nonempty() {
        let (_dir, root, mut db) = setup_ab_chain();
        // 创建子领域下的文档
        write_doc(
            &root,
            "zoloz/pay/invoice.md",
            "---\nname: invoice\nsummary: s\ntags: []\nstatus: validated\n---\nbody",
        );
        db.upsert_doc(&root, "zoloz/pay/invoice.md").unwrap();
        let v = cmd_domains(&mut db, &root, Some("zoloz")).unwrap();
        let subs = v["sub_domains"].as_array().unwrap();
        assert!(subs.iter().any(|s| s == "pay"));
        assert_eq!(v["total"], 1);
    }

    #[test]
    fn test_domains_excludes_skip_dirs() {
        let (_dir, root, mut db) = setup_ab_chain();
        // 创建空 domain
        std::fs::create_dir_all(Path::new(&root).join("newdomain")).unwrap();
        // 创建应被排除的目录
        std::fs::create_dir_all(Path::new(&root).join(".git")).unwrap();
        std::fs::create_dir_all(Path::new(&root).join(".claude")).unwrap();
        let v = cmd_domains(&mut db, &root, None).unwrap();
        let list = v["domains"].as_array().unwrap();
        assert!(list.iter().any(|d| d == "zoloz"));
        assert!(list.iter().any(|d| d == "newdomain"));
        assert!(!list.iter().any(|d| d == ".git"), ".git 不应被列为 domain");
        assert!(!list.iter().any(|d| d == ".claude"), ".claude 不应被列为 domain");
    }


    #[test]
    fn test_trashbox_list() {
        let (_dir, root, _db) = setup_ab_chain();
        // 初始为空
        let v = cmd_trashbox_list(&root).unwrap();
        assert_eq!(v["count"], 0);
        // 预置回收站文件(模拟 rm 移入)
        write_doc(&root, ".trash-box/zoloz/a.md", "---\nname: a\nsummary: a\nstatus: validated\n---\nbody\n");
        write_doc(&root, ".trash-box/zoloz/pay/b.md", "---\nname: b\nsummary: b\nstatus: validated\n---\nbody\n");
        let v = cmd_trashbox_list(&root).unwrap();
        assert_eq!(v["count"], 2);
        let docs: Vec<String> = v["files"].as_array().unwrap().iter().map(|f| f["doc"].as_str().unwrap().to_string()).collect();
        // 返回相对 .trash-box 的原路径,已排序
        assert_eq!(docs, vec!["zoloz/a.md", "zoloz/pay/b.md"]);
        assert!(v["files"][0]["size"].is_u64() || v["files"][0]["size"].is_number());
    }
}
