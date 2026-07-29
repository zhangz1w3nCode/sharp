use std::process::Command;

fn akb_bin() -> String {
    env!("CARGO_BIN_EXE_akb").to_string()
}

fn init_temp_kb() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let kb_root = dir.path().to_str().expect("path is valid utf-8");
    let output = Command::new(akb_bin())
        .arg("--kb-root")
        .arg(kb_root)
        .arg("init")
        .arg("testdomain")
        .arg("--summary")
        .arg("root summary")
        .arg("--content")
        .arg("root body")
        .output()
        .expect("failed to run akb init");
    assert!(output.status.success(), "akb init failed: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("init output is not valid JSON");
    assert!(v.get("command").is_none(), "output should not contain command field");
    assert_eq!(v["domain"], "testdomain");
    dir
}

fn run_akb(kb_root: &str, args: &[&str]) -> serde_json::Value {
    let output = Command::new(akb_bin())
        .arg("--kb-root")
        .arg(kb_root)
        .args(args)
        .output()
        .expect("failed to run akb");
    assert!(
        output.status.success(),
        "akb {} failed: stderr={} stdout={}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("output is not valid JSON");
    assert!(v.get("command").is_none(), "output should not contain command field");
    v
}

#[test]
fn test_cli_stats_output_format() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    let v = run_akb(kb_root, &["stats"]);
    assert!(v["total_documents"].is_number());
    assert!(v["health_score"].is_number());
    assert!(v.get("command").is_none());
}

#[test]
fn test_cli_add_and_links() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();

    let v = run_akb(kb_root, &[
        "add", "testdomain/sub.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "sub summary",
        "--content", "sub body",
    ]);
    assert_eq!(v["created"], true);
    assert_eq!(v["doc"], "testdomain/sub.md");

    let v = run_akb(kb_root, &["links", "--from", "testdomain/testdomain.md"]);
    assert_eq!(v["count"], 1);
    assert_eq!(v["related"][0]["doc"], "testdomain/sub.md");
    assert!(v["related"][0]["exists"].as_bool().unwrap_or(false));
}

#[test]
fn test_cli_error_path_outputs_valid_json() {
    let output = Command::new(akb_bin())
        .arg("--kb-root")
        .arg("/nonexistent_kb_path_for_test")
        .arg("stats")
        .output()
        .expect("failed to run akb");
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
    let v = run_akb(kb_root, &["doctor"]);
    assert!(v["summary"]["total_documents"].is_number());
    assert!(v.get("command").is_none());
}

#[test]
fn test_cli_index_operations() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();

    // add a doc first so index has content
    run_akb(kb_root, &[
        "add", "testdomain/sub.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "sub", "--content", "body",
    ]);

    // index --build
    let v = run_akb(kb_root, &["index", "--build"]);
    assert!(v["indexed"].is_number(), "build should return indexed count");
    assert_eq!(v["mode"], "build");

    // index --status
    let v = run_akb(kb_root, &["index", "--status"]);
    assert!(v["status"]["indexed_count"].is_number());
    assert_eq!(v["mode"], "status");

    // index --tree
    let v = run_akb(kb_root, &["index", "--tree"]);
    assert!(v["tree"].is_object());
    assert!(v["total"].is_number());

    // index --flat
    let v = run_akb(kb_root, &["index", "--flat"]);
    assert!(v["documents"].is_array());
    assert!(v["total"].is_number());
    let docs = v["documents"].as_array().unwrap();
    assert!(docs.len() >= 2, "should have root + sub doc");
}

#[test]
fn test_cli_search_and_show() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    run_akb(kb_root, &[
        "add", "testdomain/sub.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "sub doc about tauri", "--content", "this doc mentions tauri framework",
    ]);

    // search
    let v = run_akb(kb_root, &["search", "tauri", "--top", "5"]);
    assert!(v["matches"].is_array());
    assert!(v["total_files"].is_number());
    let matches = v["matches"].as_array().unwrap();
    assert!(!matches.is_empty(), "should find tauri in docs");

    // show --summary
    let v = run_akb(kb_root, &["show", "testdomain/sub.md", "--summary"]);
    assert_eq!(v["doc"], "testdomain/sub.md");
    assert!(v["name"].is_string() || v["name"].is_null());
    assert!(v["summary"].is_string() || v["summary"].is_null());

    // show (full)
    let v = run_akb(kb_root, &["show", "testdomain/sub.md"]);
    assert_eq!(v["doc"], "testdomain/sub.md");
    assert!(v["body"].is_string());
    assert!(v["has_frontmatter"].is_boolean());
}

#[test]
fn test_cli_tags_and_traverse() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    run_akb(kb_root, &[
        "add", "testdomain/a.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "a", "--content", "a body", "--tags", "tagA", "--tags", "tagB",
    ]);
    run_akb(kb_root, &[
        "add", "testdomain/b.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "b", "--content", "b body", "--tags", "tagB", "--label", "关联",
    ]);

    // tags (list all)
    let v = run_akb(kb_root, &["tags"]);
    assert!(v["total_tags"].is_number());
    assert!(v["tags"].is_object());

    // tags (specific)
    let v = run_akb(kb_root, &["tags", "tagB"]);
    assert!(v["count"].is_number());
    let docs = v["documents"].as_array().unwrap();
    assert!(docs.len() >= 2, "tagB should be in both a.md and b.md");

    // traverse
    let v = run_akb(kb_root, &["traverse", "--from", "testdomain/testdomain.md", "-j", "2"]);
    assert!(v["total_paths"].is_number());
    let paths = v["paths"].as_array().unwrap();
    assert!(!paths.is_empty(), "should have traversal paths");

    // traverse --bidir
    let v = run_akb(kb_root, &["traverse", "--from", "testdomain/testdomain.md", "-j", "2", "--bidir"]);
    assert!(v["total_paths"].as_u64().unwrap_or(0) > 0);

    // traverse --label-filter
    let v = run_akb(kb_root, &["traverse", "--from", "testdomain/testdomain.md", "-j", "2", "--label-filter", "关联"]);
    let filtered = v["paths"].as_array().unwrap();
    assert!(!filtered.is_empty(), "should find paths with label '关联'");
}

#[test]
fn test_cli_update_and_verify() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    run_akb(kb_root, &[
        "add", "testdomain/sub.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "original", "--content", "original body",
    ]);

    // update --append
    let v = run_akb(kb_root, &["update", "testdomain/sub.md", "--append", "APPENDED TEXT"]);
    assert!(v["changes"].as_array().unwrap().iter().any(|c| c == "append"));

    // update --summary
    let v = run_akb(kb_root, &["update", "testdomain/sub.md", "--summary", "NEW SUMMARY"]);
    assert!(v["changes"].as_array().unwrap().iter().any(|c| c == "summary"));

    // update --name
    let v = run_akb(kb_root, &["update", "testdomain/sub.md", "--name", "NewName"]);
    assert!(v["changes"].as_array().unwrap().iter().any(|c| c == "name"));

    // update --tags
    let v = run_akb(kb_root, &["update", "testdomain/sub.md", "--tags", "newtag1", "--tags", "newtag2"]);
    assert!(v["changes"].as_array().unwrap().iter().any(|c| c == "tags"));

    // update --add-link
    run_akb(kb_root, &[
        "add", "testdomain/target.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "target", "--content", "target body",
    ]);
    let v = run_akb(kb_root, &["update", "testdomain/sub.md", "--add-link", "--to", "testdomain/target.md", "--label", "ref"]);
    assert!(v["changes"].as_array().unwrap().iter().any(|c| c.as_str().unwrap_or("").starts_with("add-link")));

    // verify all changes with show
    let v = run_akb(kb_root, &["show", "testdomain/sub.md"]);
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
    run_akb(kb_root, &[
        "add", "testdomain/sub.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "sub", "--content", "sub body",
    ]);

    // stats before rm
    let before = run_akb(kb_root, &["stats"]);
    let before_count = before["total_documents"].as_u64().unwrap_or(0);

    // rm
    let v = run_akb(kb_root, &["rm", "testdomain/sub.md"]);
    assert_eq!(v["deleted"], false, "rm should not delete file");
    assert_eq!(v["doc"], "testdomain/sub.md");
    assert!(v["inlinks"].is_array(), "rm should report inlinks");

    // stats after rm (index count should decrease)
    let after = run_akb(kb_root, &["stats"]);
    let after_count = after["total_documents"].as_u64().unwrap_or(0);
    assert!(after_count < before_count, "index count should decrease after rm");

    // file still exists on disk
    let file_path = std::path::Path::new(kb_root).join("testdomain/sub.md");
    assert!(file_path.exists(), "file should still exist on disk");

    // index --build restores it
    run_akb(kb_root, &["index", "--build"]);
    let rebuilt = run_akb(kb_root, &["stats"]);
    let rebuilt_count = rebuilt["total_documents"].as_u64().unwrap_or(0);
    assert_eq!(rebuilt_count, before_count, "build should restore index to full count");
}

#[test]
fn test_cli_add_batch() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();

    // create batch JSON file
    let batch_path = std::path::Path::new(kb_root).join("batch.json");
    let batch_json = r#"[{"doc_path":"testdomain/b1.md","link_from":"testdomain/testdomain.md","label":"batch1","name":"B1","summary":"b1 summary","content":"b1 body"},{"doc_path":"testdomain/b2.md","link_from":"testdomain/testdomain.md","label":"batch2","name":"B2","summary":"b2 summary","content":"b2 body"}]"#;
    std::fs::write(&batch_path, batch_json).unwrap();

    let v = run_akb(kb_root, &["add-batch", "--from-file", batch_path.to_str().unwrap()]);
    assert_eq!(v["total"], 2);
    assert_eq!(v["created"], 2);
    assert_eq!(v["failed"], 0);
    assert!(v["created_details"].is_array());
    assert_eq!(v["created_details"][0]["doc"], "testdomain/b1.md");
    assert_eq!(v["created_details"][1]["doc"], "testdomain/b2.md");

    // verify files exist
    assert!(std::path::Path::new(kb_root).join("testdomain/b1.md").exists());
    assert!(std::path::Path::new(kb_root).join("testdomain/b2.md").exists());

    // verify with index --flat
    let v = run_akb(kb_root, &["index", "--flat"]);
    let docs = v["documents"].as_array().unwrap();
    assert!(docs.iter().any(|d| d == "testdomain/b1.md"), "b1 should be in flat listing");
    assert!(docs.iter().any(|d| d == "testdomain/b2.md"), "b2 should be in flat listing");
}

#[test]
fn test_cli_links_reverse() {
    let dir = init_temp_kb();
    let kb_root = dir.path().to_str().unwrap();
    run_akb(kb_root, &[
        "add", "testdomain/sub.md",
        "--link-from", "testdomain/testdomain.md",
        "--summary", "sub", "--content", "body", "--label", "child",
    ]);

    // forward links from root
    let v = run_akb(kb_root, &["links", "--from", "testdomain/testdomain.md"]);
    assert_eq!(v["direction"], "out");
    assert_eq!(v["count"], 1);

    // reverse links to sub (who points to sub?)
    let v = run_akb(kb_root, &["links", "--from", "testdomain/sub.md", "--reverse"]);
    assert_eq!(v["direction"], "in");
    let related = v["related"].as_array().unwrap();
    assert!(!related.is_empty(), "sub should have inlink from root");
    assert_eq!(related[0]["doc"], "testdomain/testdomain.md");
    assert_eq!(related[0]["label"], "child");
}
