use akb_core::commands::write::{cmd_init, cmd_add, cmd_review, cmd_update};
use akb_core::commands::search::cmd_search;
use akb_core::db::IndexDb;

#[test]
fn test_review_writes_to_index() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_str().unwrap();

    // 1. init + add (status: pending)
    cmd_init(root, "test", "root", vec![], "root body").unwrap();
    let mut db = IndexDb::open(root).unwrap();
    db.full_rebuild(root).unwrap();
    cmd_add(&mut db, root, "test/article.md", "test/test.md", None, "article summary", vec!["tech".to_string()], "akb framework content").unwrap();

    // 2. 搜索 (pending 应搜不到)
    let v = cmd_search(&mut db, root, "akb", None, 0).unwrap();
    assert!(v["matches"].as_array().unwrap().is_empty(), "pending不应被搜索到");

    // 3. 索引中无文档
    let docs = db.all_docs_meta().unwrap();
    assert!(docs.is_empty(), "pending不应在索引中");

    // 4. cmd_review (pending -> validated)
    let r = cmd_review(&mut db, root, "test/article.md").unwrap();
    assert_eq!(r["status"], "validated");
    assert_eq!(r["already_validated"], false);

    // 5. 搜索 (应搜到)
    let v = cmd_search(&mut db, root, "akb", None, 0).unwrap();
    assert_eq!(v["matches"].as_array().unwrap().len(), 1, "review后应可搜索");

    // 6. 索引中现在有 article.md
    let docs = db.all_docs_meta().unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].path, "test/article.md");

    // 7. 文件含 status: validated
    let content = std::fs::read_to_string(format!("{root}/test/article.md")).unwrap();
    assert!(content.contains("status: validated"));

    // 8. 再次 review (already_validated)
    let r2 = cmd_review(&mut db, root, "test/article.md").unwrap();
    assert_eq!(r2["already_validated"], true);
}

#[test]
fn test_update_after_review_resets_to_pending() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_str().unwrap();

    cmd_init(root, "test", "root", vec![], "body").unwrap();
    let mut db = IndexDb::open(root).unwrap();
    db.full_rebuild(root).unwrap();
    cmd_add(&mut db, root, "test/sub.md", "test/test.md", None, "sub", vec![], "original content").unwrap();

    // review -> validated
    cmd_review(&mut db, root, "test/sub.md").unwrap();
    let v = cmd_search(&mut db, root, "original", None, 0).unwrap();
    assert_eq!(v["matches"].as_array().unwrap().len(), 1, "review后应搜到");

    // update -> 回退 pending
    cmd_update(&mut db, root, "test/sub.md", Some("modified content"), None, None, None, vec![], false, None, None).unwrap();
    let v = cmd_search(&mut db, root, "modified", None, 0).unwrap();
    assert!(v["matches"].as_array().unwrap().is_empty(), "update后pending不应搜到");

    // 文件 status 回退
    let content = std::fs::read_to_string(format!("{root}/test/sub.md")).unwrap();
    assert!(content.contains("status: pending"), "update后应回退pending");
}

#[test]
fn test_review_restores_links() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_str().unwrap();

    // 1. init root (pending)
    cmd_init(root, "test", "root", vec![], "root body").unwrap();
    let mut db = IndexDb::open(root).unwrap();
    db.full_rebuild(root).unwrap();

    // 2. add child linked from root (pending)
    cmd_add(&mut db, root, "test/child.md", "test/test.md", None, "child summary", vec!["tech".to_string()], "child body").unwrap();

    // 3. review root -> validated
    cmd_review(&mut db, root, "test/test.md").unwrap();

    // 4. review child -> validated
    cmd_review(&mut db, root, "test/child.md").unwrap();

    // 5. root's outlinks include child (link restored by parent re-upsert)
    let out = db.outlinks("test/test.md").unwrap();
    assert!(out.iter().any(|(t, _)| t == "test/child.md"), "root should have outlink to child after review");

    // 6. child's inlinks include root (link restored)
    let in_ = db.inlinks("test/child.md").unwrap();
    assert!(in_.iter().any(|(s, _)| s == "test/test.md"), "child should have inlink from root after review");
}
