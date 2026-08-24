//! graph_petgraph.rs - 基于 petgraph 的知识库图算法。

use std::collections::{HashMap, HashSet, VecDeque};

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;

use crate::db::IndexDb;
use crate::graph::{is_root_doc, TraversePath};

/// 基于 petgraph DiGraph 的知识库图。
pub struct KbGraph {
    pub graph: DiGraph<String, Option<String>>,
    pub node_index: HashMap<String, NodeIndex>,
}

impl KbGraph {
    /// 从索引数据库构建图。
    pub fn from_index(db: &IndexDb) -> Result<Self, rusqlite::Error> {
        let mut graph = DiGraph::new();
        let mut node_index: HashMap<String, NodeIndex> = HashMap::new();

        // 确保所有文档都成为节点(即使无链接)
        for doc in db.all_docs_meta()? {
            node_index
                .entry(doc.path.clone())
                .or_insert_with(|| graph.add_node(doc.path));
        }

        // 添加边
        for (source, target, relation) in db.all_links()? {
            let s_idx = *node_index
                .entry(source.clone())
                .or_insert_with(|| graph.add_node(source));
            let t_idx = *node_index
                .entry(target.clone())
                .or_insert_with(|| graph.add_node(target));
            graph.add_edge(s_idx, t_idx, relation);
        }

        Ok(KbGraph { graph, node_index })
    }

    /// 图中是否包含指定文档。
    pub fn contains(&self, doc: &str) -> bool {
        self.node_index.contains_key(doc)
    }

    /// 返回邻居节点索引和边关系(去重)。
    fn neighbor_indices(
        &self,
        node: NodeIndex,
        bidir: bool,
    ) -> Vec<(NodeIndex, Option<String>)> {
        let mut result: Vec<(NodeIndex, Option<String>)> = Vec::new();
        let mut seen: HashSet<(NodeIndex, Option<String>)> = HashSet::new();

        for edge in self.graph.edges_directed(node, Direction::Outgoing) {
            let key = (edge.target(), edge.weight().clone());
            if seen.insert(key.clone()) {
                result.push(key);
            }
        }

        if bidir {
            for edge in self.graph.edges_directed(node, Direction::Incoming) {
                let key = (edge.source(), edge.weight().clone());
                if seen.insert(key.clone()) {
                    result.push(key);
                }
            }
        }

        result
    }

    /// BFS 遍历,按 target 去重,保留 via_root 标记。
    pub fn traverse(
        &self,
        start: &str,
        max_hops: usize,
        bidir: bool,
        relation_filter: Option<&str>,
    ) -> Vec<TraversePath> {
        if !self.contains(start) {
            return Vec::new();
        }

        let mut all_paths: Vec<Vec<(String, Option<String>)>> = Vec::new();
        let mut queue: VecDeque<Vec<(String, Option<String>)>> = VecDeque::new();
        queue.push_back(vec![(start.to_string(), None)]);

        while let Some(path) = queue.pop_front() {
            let current_hops = path.len() - 1;
            if current_hops >= max_hops {
                if current_hops >= 1 {
                    all_paths.push(path);
                }
                continue;
            }

            let current_node = &path[path.len() - 1].0;
            let current_idx = self.node_index[current_node];
            let visited_in_path: HashSet<&str> =
                path.iter().map(|(n, _)| n.as_str()).collect();
            let mut extended = false;

            for (neighbor_idx, relation) in self.neighbor_indices(current_idx, bidir) {
                let neighbor = self.graph[neighbor_idx].clone();
                if visited_in_path.contains(neighbor.as_str()) {
                    continue;
                }
                let mut new_path = path.clone();
                new_path.push((neighbor, relation));
                queue.push_back(new_path);
                extended = true;
            }

            if !extended && current_hops >= 1 {
                all_paths.push(path);
            }
        }

        // relation_filter
        if let Some(filter) = relation_filter {
            all_paths.retain(|p| {
                p.iter()
                    .skip(1)
                    .any(|(_, l)| l.as_deref() == Some(filter))
            });
        }

        // 按 target 去重:优先更短路径,同长度时优先更多带标签边
        let mut by_target: HashMap<String, (usize, usize, Vec<(String, Option<String>)>)> =
            HashMap::new();
        for p in all_paths {
            let target = p[p.len() - 1].0.clone();
            let hops = p.len() - 1;
            let relation_count = p.iter().skip(1).filter(|(_, l)| l.is_some()).count();
            match by_target.get(&target) {
                None => {
                    by_target.insert(target, (hops, relation_count, p));
                }
                Some((ex_hops, ex_relations, _)) => {
                    if hops < *ex_hops || (hops == *ex_hops && relation_count > *ex_relations) {
                        by_target.insert(target, (hops, relation_count, p));
                    }
                }
            }
        }

        let mut result: Vec<TraversePath> = Vec::new();
        for (_target, (hops, _, p)) in by_target {
            let nodes: Vec<String> = p.iter().map(|(n, _)| n.clone()).collect();
            let relations: Vec<Option<String>> =
                p.iter().skip(1).map(|(_, l)| l.clone()).collect();
            let via_root = if nodes.len() >= 3 {
                nodes[1..nodes.len() - 1]
                    .iter()
                    .any(|mid| is_root_doc(mid))
            } else {
                false
            };
            result.push(TraversePath {
                hops,
                path: nodes,
                relations,
                via_root,
            });
        }
        result.sort_by(|a, b| a.hops.cmp(&b.hops).then(a.via_root.cmp(&b.via_root)));
        result
    }

    /// 从 start 出发可达的节点集合(不含 start 自身)。
    pub fn reachable_set(
        &self,
        start: &str,
        max_hops: Option<usize>,
        bidir: bool,
    ) -> HashSet<String> {
        let start_idx = match self.node_index.get(start) {
            Some(idx) => *idx,
            None => return HashSet::new(),
        };

        let mut visited: HashSet<NodeIndex> = HashSet::new();
        visited.insert(start_idx);
        let mut queue: Vec<NodeIndex> = vec![start_idx];
        let mut hops = 0;

        while !queue.is_empty() {
            if let Some(max) = max_hops {
                if hops >= max {
                    break;
                }
            }
            let mut next_queue: Vec<NodeIndex> = Vec::new();
            for node in &queue {
                for (neighbor_idx, _) in self.neighbor_indices(*node, bidir) {
                    if !visited.contains(&neighbor_idx) {
                        visited.insert(neighbor_idx);
                        next_queue.push(neighbor_idx);
                    }
                }
            }
            queue = next_queue;
            hops += 1;
        }

        visited.remove(&start_idx);
        visited
            .iter()
            .map(|idx| self.graph[*idx].clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::IndexDb;
    use std::io::Write;

    fn write_doc(root: &str, rel: &str, content: &str) {
        let abs = std::path::Path::new(root).join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut f = std::fs::File::create(&abs).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn test_traverse_and_reachable() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().to_string();
        write_doc(
            &root,
            "zoloz/zoloz.md",
            "---\nname: zoloz\nsummary: root\ntags: []\nstatus: validated\n---\n[[`zoloz/a.md`]]\n[[`zoloz/b.md`|rel]]",
        );
        write_doc(
            &root,
            "zoloz/a.md",
            "---\nname: a\nsummary: a\ntags: []\nstatus: validated\n---\n[[`zoloz/c.md`]]",
        );
        write_doc(
            &root,
            "zoloz/b.md",
            "---\nname: b\nsummary: b\ntags: []\nstatus: validated\n---\nbody",
        );
        write_doc(
            &root,
            "zoloz/c.md",
            "---\nname: c\nsummary: c\ntags: []\nstatus: validated\n---\nbody",
        );

        let mut db = IndexDb::open(&root).unwrap();
        db.full_rebuild(&root).unwrap();
        let graph = KbGraph::from_index(&db).unwrap();

        let paths = graph.traverse("zoloz/zoloz.md", 2, false, None);
        assert_eq!(paths.len(), 2);

        let reachable = graph.reachable_set("zoloz/zoloz.md", None, false);
        assert!(reachable.contains("zoloz/a.md"));
        assert!(reachable.contains("zoloz/c.md"));
    }
}
