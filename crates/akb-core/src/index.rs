//! index.rs - 文件系统扫描与树形结构生成。

use std::collections::BTreeMap;
use std::path::Path;

use walkdir::WalkDir;

/// 跳过的目录名(版本控制/工具目录)。
const SKIP_DIRS: &[&str] = &[".git", ".claude", ".svn", ".hg"];

/// 扫描 kb_root_abs 下所有 .md 文件,返回相对路径列表(已排序)。
///
/// 跳过 .git/.claude/.svn/.hg 目录;跳过 INDEX.md 自身。
pub fn scan_files(kb_root_abs: &str) -> Vec<String> {
    let root = Path::new(kb_root_abs);
    let mut files: Vec<String> = Vec::new();
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    if SKIP_DIRS.contains(&name) {
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
        let path = entry.path();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if ext != "md" {
            continue;
        }
        let rel = match path.strip_prefix(root) {
            Ok(r) => r.to_string_lossy().replace('\\', "/").to_string(),
            Err(_) => continue,
        };
        if rel == "INDEX.md" {
            continue;
        }
        files.push(rel);
    }
    files.sort();
    files
}

/// 树节点:目录(BTreeMap)或文件(None)。
type Tree = BTreeMap<String, TreeNode>;

#[derive(Clone)]
enum TreeNode {
    Dir(Tree),
    File,
}

/// 把文件路径列表构建为嵌套树。
fn build_tree(files: &[String]) -> Tree {
    let mut root: Tree = BTreeMap::new();
    for f in files {
        let parts: Vec<&str> = f.split('/').collect();
        insert_path(&mut root, &parts);
    }
    root
}

/// 递归插入路径到树中(避免 borrow checker 问题)。
fn insert_path(tree: &mut Tree, parts: &[&str]) {
    if parts.is_empty() {
        return;
    }
    if parts.len() == 1 {
        tree.insert(parts[0].to_string(), TreeNode::File);
        return;
    }
    let entry = tree
        .entry(parts[0].to_string())
        .or_insert_with(|| TreeNode::Dir(BTreeMap::new()));
    if let TreeNode::Dir(ref mut sub) = entry {
        insert_path(sub, &parts[1..]);
    } else {
        *entry = TreeNode::Dir(BTreeMap::new());
        if let TreeNode::Dir(ref mut sub) = entry {
            insert_path(sub, &parts[1..]);
        }
    }
}

/// 递归格式化树节点为 tree 命令风格字符串。
fn format_tree_node(tree: &Tree, prefix: &str, is_last: bool, name: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let new_prefix = if name.is_empty() {
        prefix.to_string()
    } else {
        let connector = if is_last { "└── " } else { "├── " };
        lines.push(format!("{}{}{}", prefix, connector, name));
        format!("{}{}", prefix, if is_last { "    " } else { "│   " })
    };

    // 目录在前,文件在后
    let mut dirs: Vec<(&String, &Tree)> = Vec::new();
    let mut files: Vec<&String> = Vec::new();
    for (k, v) in tree.iter() {
        match v {
            TreeNode::Dir(sub) => dirs.push((k, sub)),
            TreeNode::File => files.push(k),
        }
    }
    // BTreeMap 已排序,但显式 sort 保证
    dirs.sort_by(|a, b| a.0.cmp(b.0));
    files.sort();
    let total = dirs.len() + files.len();
    let mut idx = 0;
    for (k, sub) in &dirs {
        let last = idx == total - 1;
        lines.extend(format_tree_node(sub, &new_prefix, last, k));
        idx += 1;
    }
    for k in &files {
        let last = idx == total - 1;
        let connector = if last { "└── " } else { "├── " };
        lines.push(format!("{}{}{}", new_prefix, connector, k));
        idx += 1;
    }
    lines
}

/// 把文件列表格式化为树形字符串(├── └── 风格)。
pub fn format_tree(files: &[String]) -> String {
    let tree = build_tree(files);
    let lines = format_tree_node(&tree, "", true, "");
    lines.join("\n")
}

/// 把 Tree 转为 serde_json::Value(目录->Object, 文件->Null)。
fn tree_to_json(tree: &Tree) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (k, v) in tree.iter() {
        let val = match v {
            TreeNode::Dir(sub) => tree_to_json(sub),
            TreeNode::File => serde_json::Value::Null,
        };
        map.insert(k.clone(), val);
    }
    serde_json::Value::Object(map)
}

/// 从文件列表构建树并转为 JSON Value(用于 cmd_index --tree)。
pub fn tree_to_value(files: &[String]) -> serde_json::Value {
    let tree = build_tree(files);
    tree_to_json(&tree)
}

/// 生成 INDEX.md 全文内容(frontmatter + 树形结构)。
#[allow(dead_code)]
pub fn rebuild_index_content(files: &[String]) -> String {
    let tree_str = format_tree(files);
    format!(
        "---\nname: INDEX\ndescription: 知识库全局索引\ntags: [index]\n---\n```\n{}\n```\n",
        tree_str
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_doc(root: &str, rel: &str, content: &str) {
        let abs = Path::new(root).join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut f = std::fs::File::create(&abs).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn test_scan_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().to_string();
        write_doc(&root, "zoloz/zoloz.md", "body");
        write_doc(&root, "zoloz/a.md", "body");
        write_doc(&root, "INDEX.md", "index content");
        write_doc(&root, ".git/should_skip.md", "git");
        write_doc(&root, ".claude/should_skip.md", "claude");
        write_doc(&root, "notmd.txt", "txt");

        let files = scan_files(&root);
        // 跳过 INDEX.md、.git、.claude、非 .md
        assert!(files.contains(&"zoloz/zoloz.md".to_string()));
        assert!(files.contains(&"zoloz/a.md".to_string()));
        assert!(!files.contains(&"INDEX.md".to_string()));
        assert!(!files.iter().any(|f| f.contains(".git")));
        assert!(!files.iter().any(|f| f.contains(".claude")));
        assert!(!files.contains(&"notmd.txt".to_string()));
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_format_tree() {
        let files = vec![
            "zoloz/a.md".to_string(),
            "zoloz/b.md".to_string(),
            "root.md".to_string(),
        ];
        let tree = format_tree(&files);
        assert!(tree.contains("├──") || tree.contains("└──"));
        assert!(tree.contains("zoloz"));
        assert!(tree.contains("a.md"));
        assert!(tree.contains("b.md"));
        assert!(tree.contains("root.md"));
    }

    #[test]
    fn test_tree_to_value() {
        let files = vec![
            "zoloz/a.md".to_string(),
            "root.md".to_string(),
        ];
        let v = tree_to_value(&files);
        let obj = v.as_object().expect("root should be object");
        assert!(obj.contains_key("zoloz"));
        assert!(obj.contains_key("root.md"));
        // root.md 是文件 -> Null
        assert!(obj.get("root.md").unwrap().is_null());
        // zoloz 是目录 -> Object
        let sub = obj.get("zoloz").unwrap().as_object().unwrap();
        assert!(sub.contains_key("a.md"));
        assert!(sub.get("a.md").unwrap().is_null());
    }

    #[test]
    fn test_scan_files_sorted() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().to_string();
        // 故意乱序写入
        write_doc(&root, "c.md", "c");
        write_doc(&root, "a.md", "a");
        write_doc(&root, "b.md", "b");

        let files = scan_files(&root);
        assert_eq!(files, vec!["a.md", "b.md", "c.md"]);
    }
}
