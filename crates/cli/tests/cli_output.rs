use std::process::Command;

fn sharp_bin() -> String {
    env!("CARGO_BIN_EXE_sharp").to_string()
}

fn init_temp_kb() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let kb_root = dir.path().to_str().expect("path is valid utf-8");
    let output = Command::new(sharp_bin())
        .arg("--kb-root")
        .arg(kb_root)
        .arg("init")
        .arg("testdomain")
        .arg("--summary")
        .arg("root summary")
        .arg("--tags")
        .arg("[]")
        .arg("--content")
        .arg("root body")
        .output()
        .expect("failed to run sharp init");
    assert!(output.status.success(), "sharp init failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("init output is not valid JSON");
    assert!(v.get("command").is_none(), "output should not contain command field");
    assert_eq!(v["domain"], "testdomain");
    // 测试环境中将根文档标记为 validated 以便索引查询
    let root_doc = std::path::Path::new(kb_root).join("testdomain/testdomain.md");
    let content = std::fs::read_to_string(&root_doc).unwrap();
    let content = content.replace("status: pending", "status: validated");
    std::fs::write(&root_doc, content).unwrap();
    // 重建索引使 validated 文档可见
    let _ = Command::new(sharp_bin()).arg("--kb-root").arg(kb_root).arg("index").arg("--build").output();
    dir
}

/// 测试辅助:将文档 status 改为 validated 并重建索引
fn validate_doc(kb_root: &str, doc: &str) {
    let path = std::path::Path::new(kb_root).join(doc);
    let content = std::fs::read_to_string(&path).unwrap();
    let content = content.replace("status: pending", "status: validated");
    std::fs::write(&path, content).unwrap();
    let _ = Command::new(sharp_bin()).arg("--kb-root").arg(kb_root).arg("index").arg("--build").output();
}


fn run_sharp(kb_root: &str, args: &[&str]) -> serde_json::Value {
    let output = Command::new(sharp_bin())
        .arg("--kb-root")
        .arg(kb_root)
        .args(args)
        .output()
        .expect("failed to run sharp");
    assert!(
        output.status.success(),
        "sharp {} failed: stderr={} stdout={}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("output is not valid JSON");
    assert!(v.get("command").is_none(), "output should not contain command field");
    v
}

/// 运行 sharp 命令并期望失败,返回 JSON error 输出。
fn run_sharp_err(kb_root: &str, args: &[&str]) -> serde_json::Value {
    let output = Command::new(sharp_bin())
        .arg("--kb-root")
        .arg(kb_root)
        .args(args)
        .output()
        .expect("failed to run sharp");
    assert!(
        !output.status.success(),
        "sharp {} should have failed but succeeded: stdout={}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("error output should be valid JSON");
    assert!(v["error"].is_string(), "error output should have error field");
    v
}

#[test]
fn test_cli_stats_output_format() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    let v = run_sharp(kb_root, &["stats"]);
    assert!(v["total_documents"].is_number());
    assert!(v["health_score"].is_number());
    assert!(v.get("command").is_none());
}

#[test]
fn test_cli_add_and_links() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();

    let v = run_sharp(kb_root, &[
        "add", "testdomain/sub.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "sub summary",
        "--content", "sub body",
        "--tags", "[]",
    ]);
    assert_eq!(v["created"], true);
    assert_eq!(v["doc"], "testdomain/sub.md");
    validate_doc(kb_root, "testdomain/sub.md");

    let v = run_sharp(kb_root, &["links", "--from", "testdomain/testdomain.md"]);
    assert_eq!(v["count"], 1);
    assert_eq!(v["related"][0]["doc"], "testdomain/sub.md");
    assert!(v["related"][0]["exists"].as_bool().unwrap_or(false));
}

#[test]
fn test_cli_error_path_outputs_valid_json() {
    let output = Command::new(sharp_bin())
        .arg("--kb-root")
        .arg("/nonexistent_kb_path_for_test")
        .arg("stats")
        .output()
        .expect("failed to run sharp");
    assert!(!output.status.success(), "should fail on non-existent kb");
    assert_eq!(output.status.code(), Some(2));
    let stderr_or_stdout = if !output.stdout.is_empty() {
        String::from_utf8_lossy(&output.stdout)
    } else {
        String::from_utf8_lossy(&output.stderr)
    };
    let v: serde_json::Value = serde_json::from_str(&stderr_or_stdout)
        .expect("error output should be valid JSON");
    assert!(v["error"].is_string(), "error output should have error field");
    assert!(v.get("command").is_none());
}

#[test]
fn test_cli_doctor_output() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    let v = run_sharp(kb_root, &["doctor"]);
    assert!(v["summary"]["total_documents"].is_number());
    assert!(v.get("command").is_none());
}

#[test]
fn test_cli_index_operations() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();

    // add a doc first so index has content
    run_sharp(kb_root, &[
        "add", "testdomain/sub.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "sub", "--content", "body", "--tags", "[]",
    ]);

    validate_doc(kb_root, "testdomain/sub.md");

    // index --build
    let v = run_sharp(kb_root, &["index", "--build"]);
    assert!(v["indexed"].is_number(), "build should return indexed count");
    assert_eq!(v["mode"], "build");

    // index --status
    let v = run_sharp(kb_root, &["index", "--status"]);
    assert!(v["status"]["indexed_count"].is_number());
    assert_eq!(v["mode"], "status");

    // index --tree
    let v = run_sharp(kb_root, &["index", "--tree"]);
    assert!(v["tree"].is_object());
    assert!(v["total"].is_number());

    // index --flat
    let v = run_sharp(kb_root, &["index", "--flat"]);
    assert!(v["documents"].is_array());
    assert!(v["total"].is_number());
    let docs = v["documents"].as_array().unwrap();
    assert!(docs.len() >= 2, "should have root + sub doc");
}

#[test]
fn test_cli_search_and_show() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    run_sharp(kb_root, &[
        "add", "testdomain/sub.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "sub doc about tauri", "--content", "this doc mentions tauri framework", "--tags", "[]",
    ]);

    // search
    let v = run_sharp(kb_root, &["search", "tauri", "--top", "5"]);
    assert!(v["matches"].is_array());
    assert!(v["total_files"].is_number());
    let matches = v["matches"].as_array().unwrap();
    // 文档 status=pending,搜索过滤掉未审核文档,因此 matches 应为空
    assert!(matches.is_empty(), "pending docs should not appear in search");

    // show 同样受 pending 隔离:索引中查不到 pending 文档,报错
    let err = run_sharp_err(kb_root, &["show", "testdomain/sub.md", "--summary"]);
    assert!(err["error"].is_string(), "pending doc show should error");

    // 审核后(validated)show 正常
    validate_doc(kb_root, "testdomain/sub.md");

    // show --summary
    let v = run_sharp(kb_root, &["show", "testdomain/sub.md", "--summary"]);
    assert_eq!(v["doc"], "testdomain/sub.md");
    assert!(v["name"].is_string() || v["name"].is_null());
    assert!(v["summary"].is_string() || v["summary"].is_null());

    // show (full)
    let v = run_sharp(kb_root, &["show", "testdomain/sub.md"]);
    assert_eq!(v["doc"], "testdomain/sub.md");
    assert!(v["body"].is_string());
    assert!(v["has_frontmatter"].is_boolean());
}

#[test]
fn test_cli_tags_and_traverse() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    run_sharp(kb_root, &[
        "add", "testdomain/a.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "a", "--content", "a body", "--tags", "[tagA,tagB]",
    ]);
    validate_doc(kb_root, "testdomain/a.md");

    run_sharp(kb_root, &[
        "add", "testdomain/b.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "b", "--content", "b body", "--tags", "[tagB]", "--relation", "关联",
    ]);
    validate_doc(kb_root, "testdomain/b.md");

    // tags (list all)
    let v = run_sharp(kb_root, &["tags"]);
    assert!(v["total_tags"].is_number());
    assert!(v["tags"].is_object());

    // tags (specific)
    let v = run_sharp(kb_root, &["tags", "tagB"]);
    assert!(v["count"].is_number());
    let docs = v["documents"].as_array().unwrap();
    assert!(docs.len() >= 2, "tagB should be in both a.md and b.md");

    // traverse
    let v = run_sharp(kb_root, &["traverse", "--from", "testdomain/testdomain.md", "-j", "2"]);
    assert!(v["total_paths"].is_number());
    let paths = v["paths"].as_array().unwrap();
    assert!(!paths.is_empty(), "should have traversal paths");

    // traverse --bidir
    let v = run_sharp(kb_root, &["traverse", "--from", "testdomain/testdomain.md", "-j", "2", "--bidir"]);
    assert!(v["total_paths"].as_u64().unwrap_or(0) > 0);

    // traverse --relation-filter
    let v = run_sharp(kb_root, &["traverse", "--from", "testdomain/testdomain.md", "-j", "2", "--relation-filter", "关联"]);
    let filtered = v["paths"].as_array().unwrap();
    assert!(!filtered.is_empty(), "should find paths with relation '关联'");
}

#[test]
fn test_cli_update_and_verify() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    run_sharp(kb_root, &[
        "add", "testdomain/sub.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "original", "--content", "original body", "--tags", "[]",
    ]);

    // update --append
    let v = run_sharp(kb_root, &["update", "testdomain/sub.md", "--append", "APPENDED TEXT"]);
    assert!(v["changes"].as_array().unwrap().iter().any(|c| c == "append"));

    // update --summary
    let v = run_sharp(kb_root, &["update", "testdomain/sub.md", "--summary", "NEW SUMMARY"]);
    assert!(v["changes"].as_array().unwrap().iter().any(|c| c == "summary"));

    // update --name
    let v = run_sharp(kb_root, &["update", "testdomain/sub.md", "--name", "NewName"]);
    assert!(v["changes"].as_array().unwrap().iter().any(|c| c == "name"));

    // update --tags
    let v = run_sharp(kb_root, &["update", "testdomain/sub.md", "--tags", "[newtag1,newtag2]"]);
    assert!(v["changes"].as_array().unwrap().iter().any(|c| c == "tags"));

    // update --add-link
    run_sharp(kb_root, &[
        "add", "testdomain/target.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "target", "--content", "target body", "--tags", "[]",
    ]);
    let v = run_sharp(kb_root, &["update", "testdomain/sub.md", "--add-link", "--to", "testdomain/target.md", "--relation", "ref"]);
    assert!(v["changes"].as_array().unwrap().iter().any(|c| c.as_str().unwrap_or("").starts_with("add-link")));

    // update 后 status 回退 pending,verify 前需审核使 show 可见
    validate_doc(kb_root, "testdomain/sub.md");

    // verify all changes with show
    let v = run_sharp(kb_root, &["show", "testdomain/sub.md"]);
    let body = v["body"].as_str().unwrap();
    assert!(body.contains("APPENDED TEXT"), "body should contain appended text");
    assert!(body.contains("target.md"), "body should contain wiki-link to target");
    let fm = &v["frontmatter"];
    assert_eq!(fm["name"], "NewName");
    assert_eq!(fm["summary"], "NEW SUMMARY");
    assert!(fm["tags"].as_array().unwrap().contains(&serde_json::json!("newtag1")));
    assert!(fm["tags"].as_array().unwrap().contains(&serde_json::json!("newtag2")));
}

#[test]
fn test_cli_rm_behavior() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    run_sharp(kb_root, &[
        "add", "testdomain/sub.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "sub", "--content", "sub body", "--tags", "[]",
    ]);
    validate_doc(kb_root, "testdomain/sub.md");

    // stats before rm
    let before = run_sharp(kb_root, &["stats"]);
    let before_count = before["total_documents"].as_u64().unwrap_or(0);

    // rm
    let v = run_sharp(kb_root, &["rm", "testdomain/sub.md"]);
    assert_eq!(v["deleted"], true, "rm should delete file by moving to trash-box");
    assert_eq!(v["trash_path"], ".trash-box/testdomain/sub.md");
    assert_eq!(v["doc"], "testdomain/sub.md");
    assert!(v["inlinks"].is_array(), "rm should report inlinks");

    // stats after rm (index count should decrease)
    let after = run_sharp(kb_root, &["stats"]);
    let after_count = after["total_documents"].as_u64().unwrap_or(0);
    assert!(after_count < before_count, "index count should decrease after rm");

    // file moved into .trash-box, no longer in normal directory
    let file_path = std::path::Path::new(kb_root).join("testdomain/sub.md");
    let trash_path = std::path::Path::new(kb_root).join(".trash-box/testdomain/sub.md");
    assert!(!file_path.exists(), "file should be moved out of normal directory");
    assert!(trash_path.exists(), "file should exist under .trash-box");

    // INDEX.md 树不再包含已删文档
    let index_content = std::fs::read_to_string(format!("{}/INDEX.md", kb_root)).unwrap();
    assert!(!index_content.contains("sub.md"), "INDEX.md tree should not list removed doc");

    // index --build does NOT restore it (regression: 9->8->9 fixed to 9->8->8)
    run_sharp(kb_root, &["index", "--build"]);
    let rebuilt = run_sharp(kb_root, &["stats"]);
    let rebuilt_count = rebuilt["total_documents"].as_u64().unwrap_or(0);
    assert_eq!(rebuilt_count, after_count, "build should NOT restore trashed doc to index");
}

#[test]
fn test_cli_rejects_trash_box_paths() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    run_sharp(kb_root, &[
        "add", "testdomain/sub.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "sub", "--content", "sub body", "--tags", "[]",
    ]);
    validate_doc(kb_root, "testdomain/sub.md");
    run_sharp(kb_root, &["rm", "testdomain/sub.md"]);

    // .trash-box 是回收站保留目录:show/update 显式访问必须被拒绝(完整隔离)
    let v = run_sharp_err(kb_root, &["show", ".trash-box/testdomain/sub.md"]);
    assert!(v["error"].is_string(), "show should reject .trash-box path");
    let v = run_sharp_err(kb_root, &["update", ".trash-box/testdomain/sub.md", "--content", "x"]);
    assert!(v["error"].is_string(), "update should reject .trash-box path");
}

#[test]
fn test_cli_trashbox_list_and_restore() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    run_sharp(kb_root, &[
        "add", "testdomain/sub.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "sub", "--content", "sub body", "--tags", "[]",
    ]);
    validate_doc(kb_root, "testdomain/sub.md");

    // rm 后:回收站 list 能看到,root 指向 sub 的链接断链
    run_sharp(kb_root, &["rm", "testdomain/sub.md"]);
    let list = run_sharp(kb_root, &["trashbox", "list"]);
    assert_eq!(list["count"], 1);
    assert_eq!(list["files"][0]["doc"], "testdomain/sub.md");
    let doctor = run_sharp(kb_root, &["doctor"]);
    let dangling = doctor["dangling_links"].as_array().unwrap();
    assert!(dangling.iter().any(|l| l["target"] == "testdomain/sub.md"), "rm 后应报断链");

    // restore 后:文件回原位置,索引与链接复原,断链消失
    let v = run_sharp(kb_root, &["trashbox", "restore", "testdomain/sub.md"]);
    assert_eq!(v["restored"], true);
    assert_eq!(v["indexed"], true);
    assert!(std::path::Path::new(kb_root).join("testdomain/sub.md").exists());
    let list = run_sharp(kb_root, &["trashbox", "list"]);
    assert_eq!(list["count"], 0);
    // 链接复原:root 指向 sub 的 outlink 恢复
    let links = run_sharp(kb_root, &["links", "--from", "testdomain/testdomain.md"]);
    assert!(links["related"].as_array().unwrap().iter().any(|l| l["doc"] == "testdomain/sub.md"), "restore 后 root 指向 sub 的链接应复原");
    // 断链消失:doctor 不再报告 dangling
    let doctor = run_sharp(kb_root, &["doctor"]);
    let dangling = doctor["dangling_links"].as_array().unwrap();
    assert!(!dangling.iter().any(|l| l["target"] == "testdomain/sub.md"), "restore 后断链应消失");
    // 搜索恢复命中
    let search = run_sharp(kb_root, &["search", "sub body"]);
    assert!(search["matches"].as_array().unwrap().iter().any(|r| r["doc"] == "testdomain/sub.md"), "restore 后搜索应恢复命中");
}

#[test]
fn test_cli_links_reverse() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    run_sharp(kb_root, &[
        "add", "testdomain/sub.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "sub", "--content", "body", "--tags", "[]", "--relation", "child",
    ]);
    validate_doc(kb_root, "testdomain/sub.md");

    // forward links from root
    let v = run_sharp(kb_root, &["links", "--from", "testdomain/testdomain.md"]);
    assert_eq!(v["direction"], "out");
    assert_eq!(v["count"], 1);

    // reverse links to sub (who points to sub?)
    let v = run_sharp(kb_root, &["links", "--from", "testdomain/sub.md", "--reverse"]);
    assert_eq!(v["direction"], "in");
    let related = v["related"].as_array().unwrap();
    assert!(!related.is_empty(), "sub should have inlink from root");
    assert_eq!(related[0]["doc"], "testdomain/testdomain.md");
    assert_eq!(related[0]["relation"], "child");
}

#[test]
fn test_cli_init_output() {
    let dir = tempfile::tempdir().unwrap();
    let kb_root = dir.path().to_str().unwrap();

    let output = Command::new(sharp_bin())
        .arg("--kb-root")
        .arg(kb_root)
        .arg("init")
        .arg("mydomain")
        .arg("--summary")
        .arg("root summary")
        .arg("--tags")
        .arg("[root-tag]")
        .arg("--content")
        .arg("root content")
        .output()
        .expect("failed to run sharp init");
    assert!(output.status.success(), "sharp init failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("init output is not valid JSON");

    // JSON 输出验证
    assert_eq!(v["domain"], "mydomain");
    assert_eq!(v["name"], "mydomain");
    assert!(v["created"].is_array());
    assert!(v["index_updated"].as_bool().unwrap_or(false));

    // 目录结构验证
    assert!(std::path::Path::new(kb_root).join("mydomain").is_dir());
    assert!(std::path::Path::new(kb_root).join("mydomain/mydomain.md").exists());
    assert!(std::path::Path::new(kb_root).join("INDEX.md").exists());

    // frontmatter 验证
    let content = std::fs::read_to_string(
        std::path::Path::new(kb_root).join("mydomain/mydomain.md")
    ).unwrap();
    assert!(content.contains("name: mydomain"));
    assert!(content.contains("summary: root summary"));
    assert!(content.contains("tags: [root-tag]"));
    assert!(content.contains("status: pending"));
    assert!(content.contains("root content"));
}

#[test]
fn test_cli_search_validated() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();

    // add 文档 (pending)
    run_sharp(kb_root, &[
        "add", "testdomain/sub.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "about tauri", "--content", "tauri framework docs",
        "--tags", "[]",
    ]);

    // pending 时搜索不到
    let v = run_sharp(kb_root, &["search", "tauri"]);
    assert!(v["matches"].as_array().unwrap().is_empty(), "pending should not be searchable");

    // validate 后搜索到
    validate_doc(kb_root, "testdomain/sub.md");
    let v = run_sharp(kb_root, &["search", "tauri"]);
    let matches = v["matches"].as_array().unwrap();
    assert!(!matches.is_empty(), "validated doc should be searchable");
    assert_eq!(matches[0]["doc"], "testdomain/sub.md");
}

#[test]
fn test_cli_update_resets_status() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();

    // add + validate
    run_sharp(kb_root, &[
        "add", "testdomain/sub.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "original", "--content", "original body",
        "--tags", "[]",
    ]);
    validate_doc(kb_root, "testdomain/sub.md");

    // validated 时搜索到
    let v = run_sharp(kb_root, &["search", "original"]);
    assert!(!v["matches"].as_array().unwrap().is_empty(), "validated should be searchable");

    // update 后 status 回退 pending
    run_sharp(kb_root, &["update", "testdomain/sub.md", "--content", "modified content"]);

    // 搜索不到 (pending)
    let v = run_sharp(kb_root, &["search", "modified"]);
    assert!(v["matches"].as_array().unwrap().is_empty(), "after update, pending should not be searchable");

    // 文件 status 回退 pending
    let content = std::fs::read_to_string(
        std::path::Path::new(kb_root).join("testdomain/sub.md")
    ).unwrap();
    assert!(content.contains("status: pending"), "update should reset status to pending");
}

#[test]
fn test_cli_doctor_dangling() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();

    // add 文档含 broken wiki-link
    run_sharp(kb_root, &[
        "add", "testdomain/sub.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "has broken link",
        "--content", "[[`.knowledges/testdomain/nonexistent.md`]]",
        "--tags", "[]",
    ]);
    validate_doc(kb_root, "testdomain/sub.md");

    // doctor 检测到 dangling
    let v = run_sharp(kb_root, &["doctor"]);
    let dangling = v["dangling_links"].as_array().unwrap();
    assert!(!dangling.is_empty(), "should detect dangling link to nonexistent.md");
    assert_eq!(dangling[0]["source"], "testdomain/sub.md");
    assert_eq!(dangling[0]["target"], "testdomain/nonexistent.md");
}

#[test]
fn test_cli_add_parent_not_found() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    let v = run_sharp_err(kb_root, &[
        "add", "testdomain/sub.md",
        "--link-from", "testdomain/nonexistent.md",
        "--summary", "s", "--content", "c", "--tags", "[]",
    ]);
    assert!(v["error"].as_str().unwrap().contains("parent"));
}

#[test]
fn test_cli_add_duplicate_doc() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    run_sharp(kb_root, &[
        "add", "testdomain/dup.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "s", "--content", "c", "--tags", "[]",
    ]);
    let v = run_sharp_err(kb_root, &[
        "add", "testdomain/dup.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "s", "--content", "c", "--tags", "[]",
    ]);
    assert!(v["error"].as_str().unwrap().contains("already exists"));
}

#[test]
fn test_cli_rm_not_found() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    let v = run_sharp_err(kb_root, &["rm", "testdomain/nonexistent.md"]);
    assert!(v["error"].as_str().unwrap().contains("not found"));
}

#[test]
fn test_cli_update_not_found() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    let v = run_sharp_err(kb_root, &["update", "testdomain/nonexistent.md", "--content", "new"]);
    assert!(v["error"].as_str().unwrap().contains("not found"));
}

#[test]
fn test_cli_create_domain_top_level() {
    let dir = tempfile::tempdir().unwrap();
    let kb_root = dir.path().to_str().unwrap();
    let v = run_sharp(kb_root, &["create-domain", "newdomain"]);
    assert_eq!(v["domain"], "newdomain");
    assert!(std::path::Path::new(kb_root).join("newdomain").is_dir());
    assert!(!std::path::Path::new(kb_root).join("newdomain/newdomain.md").exists());
    assert!(std::path::Path::new(kb_root).join("INDEX.md").exists());
}
#[test]
fn test_cli_create_domain_subdomain() {
    let dir = tempfile::tempdir().unwrap();
    let kb_root = dir.path().to_str().unwrap();
    run_sharp(kb_root, &["create-domain", "parent"]);
    let v = run_sharp(kb_root, &["create-domain", "parent/child"]);
    assert_eq!(v["domain"], "parent/child");
    assert!(std::path::Path::new(kb_root).join("parent/child").is_dir());
    assert!(!std::path::Path::new(kb_root).join("parent/child/child.md").exists());
}

#[test]
fn test_cli_create_domain_parent_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let kb_root = dir.path().to_str().unwrap();
    let v = run_sharp_err(kb_root, &["create-domain", "parent/child"]);
    assert!(v["error"].as_str().unwrap().contains("parent domain not found"));
}


#[test]
fn test_cli_domains_list_all() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    run_sharp(kb_root, &["create-domain", "another"]);
    let v = run_sharp(kb_root, &["domains"]);
    let domains = v["domains"].as_array().unwrap();
    assert!(domains.iter().any(|d| d == "testdomain"));
    assert!(domains.iter().any(|d| d == "another"));
}

#[test]
fn test_cli_domains_sub_domains() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    run_sharp(kb_root, &["create-domain", "testdomain/sub"]);
    let v = run_sharp(kb_root, &["domains", "testdomain"]);
    assert_eq!(v["domain"], "testdomain");
    let subs = v["sub_domains"].as_array().unwrap();
    assert!(subs.iter().any(|s| s == "sub"));
}

#[test]
fn test_cli_add_domain_validation() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    // 添加文档到不存在的子领域 → 报错提示 create domain
    let v = run_sharp_err(kb_root, &[
        "add", "testdomain/nonexistent/sub.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "s", "--content", "c", "--tags", "[]",
    ]);
    assert!(v["error"].as_str().unwrap().contains("domain not found"));
    // 创建子领域后添加成功
    run_sharp(kb_root, &["create-domain", "testdomain/pay"]);
    run_sharp(kb_root, &[
        "add", "testdomain/pay/invoice.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "invoice", "--content", "inv", "--tags", "[]",
    ]);
    // frontmatter domain 字段 = 父目录路径
    let content = std::fs::read_to_string(
        std::path::Path::new(kb_root).join("testdomain/pay/invoice.md")
    ).unwrap();
    assert!(content.contains("domain: testdomain/pay"));
}

#[test]
fn test_cli_show_domain_field() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    run_sharp(kb_root, &["create-domain", "testdomain/sub"]);
    run_sharp(kb_root, &[
        "add", "testdomain/sub/sub.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "sub", "--content", "s", "--tags", "[]",
    ]);
    validate_doc(kb_root, "testdomain/sub/sub.md");
    let v = run_sharp(kb_root, &["show", "testdomain/sub/sub.md"]);
    assert_eq!(v["frontmatter"]["domain"], "testdomain/sub");
}

#[test]
fn test_cli_show_pending_isolated() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    // add 创建 pending 文档(未审核)
    run_sharp(kb_root, &[
        "add", "testdomain/sub.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "pending doc", "--content", "pending body", "--tags", "[]",
    ]);
    // pending 文档在 CLI 完全不可见:show 报 not found in index
    let err = run_sharp_err(kb_root, &["show", "testdomain/sub.md"]);
    assert_eq!(err["error"], "testdomain/sub.md not found in index");
    // validate 后可见
    validate_doc(kb_root, "testdomain/sub.md");
    let v = run_sharp(kb_root, &["show", "testdomain/sub.md"]);
    assert_eq!(v["doc"], "testdomain/sub.md");
}

#[test]
fn test_cli_show_tags_matches_tags_table() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    // 含空格 tag:改造前 tags_text split 会丢信息,改造后读 tags 表不丢
    run_sharp(kb_root, &[
        "add", "testdomain/sub.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "sub", "--content", "body", "--tags", "[alpha,beta gamma]",
    ]);
    validate_doc(kb_root, "testdomain/sub.md");
    let v = run_sharp(kb_root, &["show", "testdomain/sub.md"]);
    let tags = v["frontmatter"]["tags"].as_array().unwrap();
    let tag_strs: Vec<&str> = tags.iter().map(|t| t.as_str().unwrap()).collect();
    assert!(tag_strs.contains(&"alpha"), "tags 应含 alpha");
    assert!(tag_strs.contains(&"beta gamma"), "含空格 tag 不应丢信息");
    // 交叉验证:tags 查询结果与 show 一致
    let vt = run_sharp(kb_root, &["tags", "beta gamma"]);
    let docs = vt["documents"].as_array().unwrap();
    assert!(docs.iter().any(|d| d == "testdomain/sub.md"), "tags 查询应命中该文档");
}

#[test]
fn test_cli_index_flat_excludes_pending() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    // validated 文档
    run_sharp(kb_root, &[
        "add", "testdomain/sub.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "sub", "--content", "body", "--tags", "[]",
    ]);
    validate_doc(kb_root, "testdomain/sub.md");
    // pending 文档(不 validate)
    run_sharp(kb_root, &[
        "add", "testdomain/pending2.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "p", "--content", "p", "--tags", "[]",
    ]);
    let v = run_sharp(kb_root, &["index", "--flat"]);
    let docs = v["documents"].as_array().unwrap();
    assert!(docs.iter().any(|d| d == "testdomain/sub.md"), "应含 validated 文档");
    assert!(!docs.iter().any(|d| d == "testdomain/pending2.md"), "不应含 pending 文档");
    // 交叉验证:索引 DB 与 CLI 输出一致
    let vs = run_sharp(kb_root, &["index", "--status"]);
    assert_eq!(vs["status"]["indexed_count"], docs.len(), "indexed_count 应与 --flat 一致");
}

#[test]
fn test_cli_rename_domain() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    run_sharp(kb_root, &["create-domain", "testdomain/sub"]);
    run_sharp(kb_root, &[
        "add", "testdomain/sub/sub.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "sub", "--content", "s", "--tags", "[]",
    ]);
    run_sharp(kb_root, &["create-domain", "other"]);
    run_sharp(kb_root, &[
        "add", "other/other.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "other", "--content", "o", "--tags", "[]",
    ]);
    let other_doc = std::path::Path::new(kb_root).join("other/other.md");
    let other_text = std::fs::read_to_string(&other_doc).unwrap();
    std::fs::write(&other_doc, format!("{}\n- [[`.knowledges/testdomain/sub.md`]]\n", other_text)).unwrap();
    let v = run_sharp(kb_root, &["rename-domain", "testdomain", "renamed"]);
    assert_eq!(v["renamed"], true);
    assert!(!std::path::Path::new(kb_root).join("testdomain").exists());
    assert!(std::path::Path::new(kb_root).join("renamed").is_dir());
    assert!(std::path::Path::new(kb_root).join("renamed/sub/sub.md").exists());
    let other_new_text = std::fs::read_to_string(&other_doc).unwrap();
    assert!(other_new_text.contains("renamed/sub.md"));
    assert!(!other_new_text.contains("testdomain/"));
}

#[test]
fn test_cli_rename_domain_not_found() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    let v = run_sharp_err(kb_root, &["rename-domain", "nonexistent", "new"]);
    assert!(v["error"].as_str().unwrap().contains("not found"));
}

