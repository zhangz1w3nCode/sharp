/* fs.ts — 目录扫描封装 + 知识图谱获取：通过 Tauri invoke 调用 Rust 后端命令 */
import { invoke } from "@tauri-apps/api/core";
import { GraphNode, GraphEdge } from "./types";
import { GraphData } from "./derive";

export interface TreeNode {
  name: string;
  path: string;
  isDir: boolean;
  children?: TreeNode[];
}

interface DirEntry {
  name: string;
  path: string;
  is_dir: boolean;
}

export async function scanDir(kbRoot: string, dirPath: string): Promise<TreeNode[]> {
  let entries: DirEntry[];
  try {
    entries = await invoke<DirEntry[]>("scan_dir", { kbRoot, dirPath });
  } catch {
    return [];
  }

  return entries.map((e) => ({
    name: e.name,
    path: e.path,
    isDir: e.is_dir,
  }));
}

// ─── 全量知识图谱 ───────────────────────────────────────

interface BackendNode {
  path: string;
  name: string;
  summary: string;
  domain: string;
  tags: string[];
  status: string;
}

interface BackendEdge {
  source: string;
  target: string;
  relation: string | null;
}

interface GraphResponse {
  total_nodes: number;
  total_edges: number;
  nodes: BackendNode[];
  edges: BackendEdge[];
}

/** 获取全量知识图谱(只含 validated 文档)，返回前端 GraphData 格式 */
export async function getGraph(kbRoot: string): Promise<GraphData> {
  const res = await invoke<GraphResponse>("get_graph", { kbRoot });
  const idx = new Map(res.nodes.map((n, i) => [n.path, i]));
  const nodes: GraphNode[] = res.nodes.map((n) => ({
    id: n.path,
    title: n.name,
    summary: n.summary,
  }));
  const edges: GraphEdge[] = [];
  for (const e of res.edges) {
    const s = idx.get(e.source);
    const t = idx.get(e.target);
    if (s !== undefined && t !== undefined && s !== t) {
      edges.push({ s, t, rel: e.relation ?? "" });
    }
  }
  return { nodes, edges };
}

// ─── 按路径获取文档 ─────────────────────────────────

export interface DocResponse {
  doc: string;
  has_frontmatter: boolean;
  frontmatter: {
    name: string;
    summary: string;
    domain: string;
    tags: string[];
  };
  body: string;
}

/** 按 path 获取文档完整内容(frontmatter + body) */
export async function getDoc(
  kbRoot: string,
  doc: string,
  summary: boolean = false,
): Promise<DocResponse> {
  return invoke<DocResponse>("get_doc", { kbRoot, doc, summary });
}

export interface BacklinkItem {
  doc: string;
  relation: string | null;
  exists: boolean;
}

export interface BacklinksResponse {
  doc: string;
  direction: string;
  count: number;
  related: BacklinkItem[];
}

/** 获取文档反向链接(inlinks) */
export async function getBacklinks(kbRoot: string, doc: string): Promise<BacklinksResponse> {
  return invoke<BacklinksResponse>("get_backlinks", { kbRoot, doc });
}
