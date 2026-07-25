// ┌─────────────────────────────────────────────────────────────────────────┐
// │  临时桥接代码 — 待对接 be/ 后端后删除                                        │
// │  scan_dir / project_root 为前端文件树提供临时目录扫描能力。               │
// │  be/ 后端已有 scan_files + build_tree + tree_to_value 完整实现，            │
// │  后续前端改为调用 be/ core，此段代码将被替换。                             │
// └─────────────────────────────────────────────────────────────────────────┘

use serde::Serialize;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize)]
struct DirEntry {
    name: String,
    path: String,
    is_dir: bool,
}

const SKIP_DIRS: &[&str] = &[".git", ".claude", ".svn", ".hg"];

fn project_root(kb_root: &str) -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let mut path = cwd;
    for _ in 0..5 {
        if path.join(kb_root).exists() {
            return Some(path);
        }
        if !path.pop() {
            break;
        }
    }
    None
}

#[tauri::command]
fn scan_dir(kb_root: String, dir_path: String) -> Vec<DirEntry> {
    let root = match project_root(&kb_root) {
        Some(r) => r,
        None => return vec![],
    };
    let abs_path = if dir_path.starts_with('/') {
        PathBuf::from(&dir_path)
    } else {
        root.join(&dir_path)
    };

    let entries = match fs::read_dir(&abs_path) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    let mut nodes: Vec<DirEntry> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                return None;
            }
            let ft = e.file_type().ok()?;
            let rel = format!("{}/{}", dir_path, name);
            if ft.is_dir() {
                if SKIP_DIRS.contains(&name.as_str()) {
                    return None;
                }
                Some(DirEntry { name, path: rel, is_dir: true })
            } else if ft.is_file() {
                if name == "INDEX.md" || !name.ends_with(".md") {
                    return None;
                }
                Some(DirEntry { name: name.replace(".md", ""), path: rel, is_dir: false })
            } else {
                None
            }
        })
        .collect();

    nodes.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    nodes
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    builder = builder.plugin(tauri_plugin_fs::init());

    #[cfg(debug_assertions)]
    {
        builder = builder.plugin(tauri_plugin_pilot::init());
    }

    builder
        .invoke_handler(tauri::generate_handler![scan_dir])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
