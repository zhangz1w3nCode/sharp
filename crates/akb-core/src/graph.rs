//! graph.rs - 图相关公共符号(供 petgraph 模块与 commands 复用)。
//!
//! BFS/邻接表等图算法实现在 graph_petgraph.rs(基于索引,不扫文件系统)。

use crate::parser::normalize_path;

use serde::Serialize;

/// 判断是否为根文档:顶层文档,或文件名==父目录名+.md。
pub fn is_root_doc(path: &str) -> bool {
    if !path.contains('/') {
        return true;
    }
    let parts: Vec<&str> = path.split('/').collect();
    let fname = parts[parts.len() - 1];
    let parent_dir = if parts.len() >= 2 {
        parts[parts.len() - 2]
    } else {
        ""
    };
    fname == format!("{}.md", parent_dir)
}

/// 把命令行传入的文档路径归一化。
pub fn norm_doc_arg(doc_arg: &str, kb_root: &str) -> String {
    normalize_path(doc_arg, kb_root)
}

/// traverse 结果中的单条路径。
#[derive(Serialize)]
pub struct TraversePath {
    pub hops: usize,
    pub path: Vec<String>,
    pub labels: Vec<Option<String>>,
    pub via_root: bool,
}
