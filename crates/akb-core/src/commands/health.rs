//! commands/health.rs - doctor + stats 子命令。

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use serde_json::{json, Value};

use crate::db::IndexDb;
use crate::graph::is_root_doc;
use crate::error::KbError;
use crate::util::round2;
use crate::graph_petgraph::KbGraph;

/// 判断 target 路径在文件系统上是否可达(支持跨库)。
///
/// 检查 kb_root_abs/target 和 project_root/target;无 .md 后缀补 .md 再查。
fn is_reachable(target: &str, kb_root_abs: &str) -> bool {
    let project_root = match Path::new(kb_root_abs).parent() {
        Some(p) => p,
        None => return false,
    };
    let mut candidates: Vec<std::path::PathBuf> = vec![
        Path::new(kb_root_abs).join(target),
        project_root.join(target),
    ];
    if !target.ends_with(".md") {
        candidates.push(Path::new(kb_root_abs).join(format!("{}.md", target)));
        candidates.push(project_root.join(format!("{}.md", target)));
    }
    candidates.iter().any(|p| p.exists())
}

/// kb doctor - 知识库健康检查。
pub fn cmd_doctor(db: &mut IndexDb, kb_root_abs: &str) -> Result<Value, KbError> {
    let docs_meta = db.all_docs_meta()
        .map_err(|e| KbError::Other(format!("docs meta query failed: {}", e)))?;
    let all_links = db.all_links()
        .map_err(|e| KbError::Other(format!("links query failed: {}", e)))?;

    let doc_paths: HashSet<String> = docs_meta.iter().map(|d| d.path.clone()).collect();
    let all_nodes: HashSet<String> = doc_paths
        .iter()
        .cloned()
        .chain(all_links.iter().map(|(_, t, _)| t.clone()))
        .collect();

    let mut outlinks: HashMap<String, Vec<(String, Option<String>)>> = HashMap::new();
    let mut inlinks: HashMap<String, Vec<(String, Option<String>)>> = HashMap::new();
    for (source, target, label) in &all_links {
        outlinks
            .entry(source.clone())
            .or_default()
            .push((target.clone(), label.clone()));
        inlinks
            .entry(target.clone())
            .or_default()
            .push((source.clone(), label.clone()));
    }

    // 1. 连通性
    let mut roots: Vec<String> = doc_paths
        .iter()
        .filter(|p| is_root_doc(p))
        .cloned()
        .collect();
    roots.sort();

    let graph = KbGraph::from_index(db)
        .map_err(|e| KbError::Other(format!("graph build failed: {}", e)))?;
    let mut reachable: HashSet<String> = HashSet::new();
    for root in &roots {
        reachable.insert(root.clone());
        for x in graph.reachable_set(root, None, true) {
            reachable.insert(x);
        }
    }
    let mut unreachable: Vec<String> = doc_paths
        .iter()
        .filter(|p| !reachable.contains(*p))
        .cloned()
        .collect();
    unreachable.sort();

    // 2. 孤儿文档
    let mut orphans: Vec<String> = Vec::new();
    for path in &doc_paths {
        if is_root_doc(path) {
            continue;
        }
        let inn = inlinks.get(path).cloned().unwrap_or_default();
        let has_real_source = inn.iter().any(|(s, _)| doc_paths.contains(s));
        if !has_real_source {
            orphans.push(path.clone());
        }
    }
    orphans.sort();

    // 3. 真断链
    let mut dangling: Vec<Value> = Vec::new();
    for doc in &docs_meta {
        for (tgt, label) in &doc.outlinks {
            if doc_paths.contains(tgt) {
                continue;
            }
            if is_reachable(tgt, kb_root_abs) {
                continue;
            }
            dangling.push(json!({
                "source": doc.path,
                "target": tgt,
                "label": label,
            }));
        }
    }
    dangling.sort_by(|a, b| {
        let sa = a["source"].as_str().unwrap_or("");
        let sb = b["source"].as_str().unwrap_or("");
        let ta = a["target"].as_str().unwrap_or("");
        let tb = b["target"].as_str().unwrap_or("");
        sa.cmp(sb).then(ta.cmp(tb))
    });

    // 孤立节点
    let mut isolated: Vec<String> = Vec::new();
    for path in &doc_paths {
        if is_root_doc(path) {
            continue;
        }
        let inn = inlinks.get(path).cloned().unwrap_or_default();
        let has_inlink = inn.iter().any(|(s, _)| doc_paths.contains(s));
        let out = outlinks.get(path).cloned().unwrap_or_default();
        let has_valid_outlink = out
            .iter()
            .any(|(t, _)| doc_paths.contains(t) || is_reachable(t, kb_root_abs));
        if !has_inlink && !has_valid_outlink {
            isolated.push(path.clone());
        }
    }
    isolated.sort();

    // 4. 重复 name
    let mut name_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for doc in &docs_meta {
        if !doc.name.is_empty() {
            name_map
                .entry(doc.name.clone())
                .or_default()
                .push(doc.path.clone());
        }
    }
    let duplicates: BTreeMap<String, Vec<String>> = name_map
        .iter()
        .filter(|(_, ps)| ps.len() > 1)
        .map(|(n, ps)| {
            let mut sorted = ps.clone();
            sorted.sort();
            (n.clone(), sorted)
        })
        .collect();

    // 5. frontmatter 缺失
    let mut missing_fm: Vec<String> = docs_meta
        .iter()
        .filter(|d| !d.has_frontmatter)
        .map(|d| d.path.clone())
        .collect();
    missing_fm.sort();

    // 6. tag 摘要
    let mut tag_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for doc in &docs_meta {
        for t in &doc.tags {
            tag_map
                .entry(t.clone())
                .or_default()
                .push(doc.path.clone());
        }
    }
    let tags_obj: BTreeMap<String, Vec<String>> = tag_map
        .iter()
        .map(|(t, ps)| {
            let mut sorted = ps.clone();
            sorted.sort();
            (t.clone(), sorted)
        })
        .collect();
    let mut single_doc_tags: Vec<Value> = tag_map
        .iter()
        .filter(|(_, ps)| ps.len() == 1)
        .map(|(t, ps)| json!({"tag": t, "doc": ps[0]}))
        .collect();
    single_doc_tags.sort_by(|a, b| {
        a["tag"].as_str().unwrap_or("").cmp(b["tag"].as_str().unwrap_or(""))
    });

    Ok(json!({
        "summary": {
            "total_documents": doc_paths.len(),
            "total_nodes_incl_dangling": all_nodes.len(),
            "unreachable": unreachable.len(),
            "orphans": orphans.len(),
            "isolated": isolated.len(),
            "dangling_links": dangling.len(),
            "duplicate_names": duplicates.len(),
            "frontmatter_missing": missing_fm.len(),
        },
        "connectivity": {
            "roots": roots,
            "unreachable": unreachable,
        },
        "orphans": orphans,
        "isolated": isolated,
        "dangling_links": dangling,
        "duplicate_names": duplicates,
        "frontmatter_missing": missing_fm,
        "tag_summary": {
            "total_tags": tag_map.len(),
            "tags": tags_obj,
            "single_doc_tags": single_doc_tags,
        },
    }))
}

/// kb stats - 知识库精简概览。
pub fn cmd_stats(db: &mut IndexDb, kb_root_abs: &str) -> Result<Value, KbError> {
    let docs_meta = db.all_docs_meta()
        .map_err(|e| KbError::Other(format!("docs meta query failed: {}", e)))?;
    let all_links = db.all_links()
        .map_err(|e| KbError::Other(format!("links query failed: {}", e)))?;

    let doc_paths: HashSet<String> = docs_meta.iter().map(|d| d.path.clone()).collect();
    let total = doc_paths.len();

    // domains / sub_domains
    let mut domains: HashSet<String> = HashSet::new();
    let mut sub_domains: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for path in &doc_paths {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() > 1 {
            domains.insert(parts[0].to_string());
            let sub = sub_domains.entry(parts[0].to_string()).or_default();
            if parts.len() > 2 && !sub.contains(&parts[1].to_string()) {
                sub.push(parts[1].to_string());
            }
        }
    }
    let mut domains: Vec<String> = domains.into_iter().collect();
    domains.sort();
    let sub_domains_sorted: BTreeMap<String, Vec<String>> = sub_domains
        .iter()
        .map(|(d, s)| {
            let mut sorted = s.clone();
            sorted.sort();
            (d.clone(), sorted)
        })
        .collect();

    // tags
    let mut all_tags: HashSet<String> = HashSet::new();
    for doc in &docs_meta {
        for t in &doc.tags {
            all_tags.insert(t.clone());
        }
    }

    // 文件大小 + last_modified
    let mut sizes: Vec<u64> = Vec::new();
    let mut last_modified_doc: Option<String> = None;
    let mut last_mtime_secs: i64 = 0;
    let mut last_mtime_nanos: u32 = 0;
    for doc in &docs_meta {
        sizes.push(doc.size_bytes);
        if doc.mtime_secs > last_mtime_secs
            || (doc.mtime_secs == last_mtime_secs && doc.mtime_nanos > last_mtime_nanos)
        {
            last_mtime_secs = doc.mtime_secs;
            last_mtime_nanos = doc.mtime_nanos;
            last_modified_doc = Some(doc.path.clone());
        }
    }
    let avg_size = if !sizes.is_empty() {
        (sizes.iter().sum::<u64>() as f64 / sizes.len() as f64).round() as u64
    } else {
        0
    };

    // inlinks/outlinks maps for counts
    let mut inlinks: HashMap<String, Vec<(String, Option<String>)>> = HashMap::new();
    let mut outlinks: HashMap<String, Vec<(String, Option<String>)>> = HashMap::new();
    for (source, target, label) in &all_links {
        outlinks
            .entry(source.clone())
            .or_default()
            .push((target.clone(), label.clone()));
        inlinks
            .entry(target.clone())
            .or_default()
            .push((source.clone(), label.clone()));
    }
    // health_score
    let graph = KbGraph::from_index(db)
        .map_err(|e| KbError::Other(format!("graph build failed: {}", e)))?;
    let roots: Vec<String> = doc_paths
        .iter()
        .filter(|p| is_root_doc(p))
        .cloned()
        .collect();
    let mut reachable: HashSet<String> = HashSet::new();
    for root in &roots {
        reachable.insert(root.clone());
        for x in graph.reachable_set(root, None, true) {
            reachable.insert(x);
        }
    }
    let unreachable_count = doc_paths.iter().filter(|p| !reachable.contains(*p)).count();

    let orphans_count = {
        let mut c = 0;
        for path in &doc_paths {
            if is_root_doc(path) {
                continue;
            }
            let inn = inlinks.get(path).cloned().unwrap_or_default();
            let has_real_source = inn.iter().any(|(s, _)| doc_paths.contains(s));
            if !has_real_source {
                c += 1;
            }
        }
        c
    };

    let dangling_count = {
        let mut c = 0;
        for doc in &docs_meta {
            for (tgt, _) in &doc.outlinks {
                if doc_paths.contains(tgt) {
                    continue;
                }
                if is_reachable(tgt, kb_root_abs) {
                    continue;
                }
                c += 1;
            }
        }
        c
    };

    let isolated_count = {
        let mut c = 0;
        for path in &doc_paths {
            if is_root_doc(path) {
                continue;
            }
            let inn = inlinks.get(path).cloned().unwrap_or_default();
            let has_inlink = inn.iter().any(|(s, _)| doc_paths.contains(s));
            let out = outlinks.get(path).cloned().unwrap_or_default();
            let has_valid_outlink = out
                .iter()
                .any(|(t, _)| doc_paths.contains(t) || is_reachable(t, kb_root_abs));
            if !has_inlink && !has_valid_outlink {
                c += 1;
            }
        }
        c
    };

    let health_score = if total > 0 {
        round2(
            (total - unreachable_count - isolated_count - dangling_count) as f64 / total as f64,
        )
    } else {
        0.0
    };
    let last_modified_time = if last_mtime_secs > 0 {
        iso8601_local(last_mtime_secs, last_mtime_nanos)
    } else {
        Value::Null
    };

    Ok(json!({
        "total_documents": total,
        "total_tags": all_tags.len(),
        "domains": domains,
        "sub_domains": sub_domains_sorted,
        "avg_size_bytes": avg_size,
        "health_score": health_score,
        "orphans": orphans_count,
        "isolated": isolated_count,
        "dangling_links": dangling_count,
        "unreachable": unreachable_count,
        "last_modified": {
            "doc": last_modified_doc,
            "mtime": last_modified_time,
        },
    }))
}
/// 把 Unix 时间戳(秒+纳秒)格式化为 ISO 8601 本地时间字符串。
fn iso8601_local(secs: i64, nanos: u32) -> Value {
    use chrono::TimeZone;
    let dt = match chrono::Local.timestamp_opt(secs, nanos) {
        chrono::LocalResult::Single(dt) => dt,
        _ => return Value::Null,
    };
    if nanos == 0 {
        Value::String(dt.format("%Y-%m-%dT%H:%M:%S").to_string())
    } else {
        Value::String(dt.format("%Y-%m-%dT%H:%M:%S%.6f").to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::write::cmd_init;
    use std::io::Write;

    fn write_doc(root: &str, rel: &str, content: &str) {
        let abs = Path::new(root).join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut f = std::fs::File::create(&abs).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    /// 全连通无孤儿 kb:root -> a -> b,b 回链 root(双向连通)。
    fn setup_connected_kb() -> (tempfile::TempDir, String, IndexDb) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().to_string();
        write_doc(
            &root,
            "zoloz/zoloz.md",
            "---\nname: zoloz\nsummary: root\ntags: [root]\nstatus: validated\n---\n[[`.knowledges/zoloz/a.md`]]",
        );
        write_doc(
            &root,
            "zoloz/a.md",
            "---\nname: a\nsummary: a\ntags: [alpha]\nstatus: validated\n---\n[[`.knowledges/zoloz/b.md`]]",
        );
        write_doc(
            &root,
            "zoloz/b.md",
            "---\nname: b\nsummary: b\ntags: [beta]\nstatus: validated\n---\n[[`.knowledges/zoloz/zoloz.md`]]",
        );
        let mut db = IndexDb::open(&root).unwrap();
        db.full_rebuild(&root).unwrap();
        (dir, root, db)
    }

    /// 空 kb(仅 init 后)。
    fn setup_empty_kb() -> (tempfile::TempDir, String, IndexDb) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().to_string();
        let result = cmd_init(&root, "zoloz", None, "root summary", None, None, vec![], "root content");
        assert!(result.is_ok());
        let db = IndexDb::open(&root).unwrap();
        (dir, root, db)
    }

    // ===== cmd_doctor =====

    #[test]
    fn test_doctor_clean_kb() {
        let (_dir, root, mut db) = setup_connected_kb();
        let result = cmd_doctor(&mut db, &root);
        assert!(result.is_ok());
        let v = result.unwrap();
        assert!(v.get("command").is_none(), "core 返回值不应含 command 字段");
        // 交叉验证:无孤儿、无断链
        let docs = db.all_docs_meta().unwrap();
        let doc_paths: HashSet<String> = docs.iter().map(|d| d.path.clone()).collect();
        let links = db.all_links().unwrap();
        // 所有 link target 都在 doc_paths 中(无 dangling)
        for (_, t, _) in &links {
            assert!(doc_paths.contains(t), "dangling link to {}", t);
        }
    }

    #[test]
    fn test_doctor_orphans() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().to_string();
        // root -> a,但 b 无入链(孤儿)
        write_doc(
            &root,
            "zoloz/zoloz.md",
            "---\nname: zoloz\nsummary: root\ntags: [root]\nstatus: validated\n---\n[[`.knowledges/zoloz/a.md`]]",
        );
        write_doc(
            &root,
            "zoloz/a.md",
            "---\nname: a\nsummary: a\ntags: [alpha]\nstatus: validated\n---\na body",
        );
        write_doc(
            &root,
            "zoloz/b.md",
            "---\nname: b\nsummary: b\ntags: [beta]\nstatus: validated\n---\nb body",
        );
        let mut db = IndexDb::open(&root).unwrap();
        db.full_rebuild(&root).unwrap();

        let result = cmd_doctor(&mut db, &root);
        assert!(result.is_ok());

        // 交叉验证:自算孤儿数
        let docs = db.all_docs_meta().unwrap();
        let doc_paths: HashSet<String> = docs.iter().map(|d| d.path.clone()).collect();
        let all_links = db.all_links().unwrap();
        let mut inlinks: HashMap<String, Vec<String>> = HashMap::new();
        for (s, t, _) in &all_links {
            inlinks.entry(t.clone()).or_default().push(s.clone());
        }
        let orphan_count: usize = doc_paths.iter().filter(|p| {
            if is_root_doc(p) { return false; }
            let inn = inlinks.get(*p).cloned().unwrap_or_default();
            !inn.iter().any(|s| doc_paths.contains(s))
        }).count();
        // b.md 是孤儿(无入链)
        assert!(orphan_count >= 1);
    }

    #[test]
    fn test_doctor_dangling() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().to_string();
        // wiki-link 指向不存在的文档
        write_doc(
            &root,
            "zoloz/zoloz.md",
            "---\nname: zoloz\nsummary: root\ntags: [root]\nstatus: validated\n---\n[[`.knowledges/zoloz/nonexistent.md`]]",
        );
        let mut db = IndexDb::open(&root).unwrap();
        db.full_rebuild(&root).unwrap();

        let result = cmd_doctor(&mut db, &root);
        assert!(result.is_ok());

        // 交叉验证:link target 不在 doc_paths 中
        let docs = db.all_docs_meta().unwrap();
        let doc_paths: HashSet<String> = docs.iter().map(|d| d.path.clone()).collect();
        for d in &docs {
            for (t, _) in &d.outlinks {
                if !doc_paths.contains(t) {
                    // 找到 dangling link
                    assert!(true);
                    return;
                }
            }
        }
        panic!("expected at least one dangling link");
    }

    #[test]
    fn test_doctor_missing_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().to_string();
        // 文档无 frontmatter
        write_doc(&root, "zoloz/zoloz.md", "[[`.knowledges/zoloz/a.md`]]");
        write_doc(
            &root,
            "zoloz/a.md",
            "---\nname: a\nsummary: a\ntags: []\nstatus: validated\n---\na body",
        );
        let mut db = IndexDb::open(&root).unwrap();
        db.full_rebuild(&root).unwrap();

        let result = cmd_doctor(&mut db, &root);
        assert!(result.is_ok());
        let v = result.unwrap();
        assert!(v.get("command").is_none(), "core 返回值不应含 command 字段");
        // 交叉调 db.all_docs_meta 验证 has_frontmatter=false
        let docs = db.all_docs_meta().unwrap();
        let root_doc = docs.iter().find(|d| d.path == "zoloz/zoloz.md").unwrap();
        assert!(!root_doc.has_frontmatter);
    }

    // ===== cmd_stats =====

    #[test]
    fn test_stats_basic() {
        let (_dir, root, mut db) = setup_connected_kb();
        let result = cmd_stats(&mut db, &root);
        assert!(result.is_ok());
        let v = result.unwrap();
        assert!(v.get("command").is_none(), "core 返回值不应含 command 字段");
    }

    #[test]
    fn test_stats_empty_kb() {
        let (_dir, root, mut db) = setup_empty_kb();
        let result = cmd_stats(&mut db, &root);
        assert!(result.is_ok());
    }

    #[test]
    fn test_stats_health_score() {
        let (_dir, root, mut db) = setup_connected_kb();
        let result = cmd_stats(&mut db, &root);
        assert!(result.is_ok());
        // 交叉调 graph.reachable_set 验证不可达数为 0
        let docs = db.all_docs_meta().unwrap();
        let doc_paths: HashSet<String> = docs.iter().map(|d| d.path.clone()).collect();
        let roots: Vec<String> = doc_paths.iter().filter(|p| is_root_doc(p)).cloned().collect();
        let graph = KbGraph::from_index(&db).unwrap();
        let mut reachable: HashSet<String> = HashSet::new();
        for root_doc in &roots {
            reachable.insert(root_doc.clone());
            for x in graph.reachable_set(root_doc, None, true) {
                reachable.insert(x);
            }
        }
        let unreachable: Vec<&String> = doc_paths.iter().filter(|p| !reachable.contains(*p)).collect();
        assert!(unreachable.is_empty(), "unreachable docs: {:?}", unreachable);
    }
}
