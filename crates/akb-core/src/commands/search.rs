//! commands/search.rs - 查询类子命令:index/links/traverse/tags/search/show。
//!
//! 读命令只读索引(写命令已实时单点增量更新);index --tree/--flat 扫文件系统。

use std::path::Path;

use serde_json::{json, Value};

use crate::db::IndexDb;
use crate::error::KbError;
use crate::util::round2;
use crate::graph::norm_doc_arg;
use crate::graph_petgraph::KbGraph;
use crate::index::{scan_files, tree_to_value};
use crate::parser::parse_frontmatter;

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

/// kb index --tree | --flat(文件系统扫描,不依赖索引)。
pub fn cmd_index(_db: &mut IndexDb, kb_root_abs: &str, flat: bool) -> Result<Value, KbError> {
    let files = scan_files(kb_root_abs);
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
        .map(|(other, label)| {
            let other_abs = Path::new(kb_root_abs)
                .join(other.replace('/', std::path::MAIN_SEPARATOR_STR));
            json!({
                "doc": other,
                "label": label,
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

/// kb traverse --from <doc> [-j N] [--bidir] [--label-filter <kw>]
pub fn cmd_traverse(
    db: &mut IndexDb,
    _kb_root_abs: &str,
    from: &str,
    jumps: usize,
    bidir: bool,
    label_filter: Option<&str>,
) -> Result<Value, KbError> {
    let doc = norm_doc_arg(from, ".knowledges");
    let graph = KbGraph::from_index(db)
        .map_err(|e| KbError::Other(format!("graph build failed: {}", e)))?;

    if !graph.contains(&doc) {
        return Err(KbError::Other(format!("{} not found in knowledge base", doc)));
    }

    let paths = graph.traverse(&doc, jumps, bidir, label_filter);
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
        "label_filter": label_filter,
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
pub fn cmd_show(kb_root_abs: &str, doc: &str, summary_only: bool) -> Result<Value, KbError> {
    let doc = norm_doc_arg(doc, ".knowledges");
    let abs = Path::new(kb_root_abs).join(doc.replace('/', std::path::MAIN_SEPARATOR_STR));
    let text = std::fs::read_to_string(&abs)
        .map_err(|_| KbError::Other(format!("{} not found in knowledge base", doc)))?;
    let (fm, body, has_fm) = parse_frontmatter(&text);
    if summary_only {
        return Ok(json!({
            "doc": doc,
            "summary": fm.summary,
            "name": fm.name,
        }));
    }
    Ok(json!({
        "doc": doc,
        "has_frontmatter": has_fm,
        "frontmatter": {
            "name": fm.name,
            "summary": fm.summary,
            "category": fm.category,
            "tags": fm.tags,
        },
        "body": body,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph_petgraph::KbGraph;
    use crate::index::scan_files;
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
        // 交叉调 scan_files 验证数量一致
        let files = scan_files(&root);
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_index_flat() {
        let (_dir, root, mut db) = setup_ab_chain();
        let result = cmd_index(&mut db, &root, true);
        assert!(result.is_ok());
        let files = scan_files(&root);
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
    fn test_traverse_label_filter() {
        let (_dir, root, mut db) = setup_abc_chain();
        // 带 "关系" 标签过滤
        let result = cmd_traverse(&mut db, &root, "zoloz/a.md", 2, false, Some("关系"));
        assert!(result.is_ok());
        // 交叉验证:filtered 路径应都含 "关系" 标签
        let graph = KbGraph::from_index(&db).unwrap();
        let paths = graph.traverse("zoloz/a.md", 2, false, Some("关系"));
        for p in &paths {
            assert!(p.labels.iter().any(|l| l.as_deref() == Some("关系")));
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
        let (_dir, root, _db) = setup_ab_chain();
        let result = cmd_show(&root, "zoloz/a.md", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_show_summary() {
        let (_dir, root, _db) = setup_ab_chain();
        let result = cmd_show(&root, "zoloz/a.md", true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_show_not_found_error() {
        let (_dir, root, _db) = setup_ab_chain();
        let result = cmd_show(&root, "zoloz/nonexistent.md", false);
        assert!(result.is_err());
    }
}
