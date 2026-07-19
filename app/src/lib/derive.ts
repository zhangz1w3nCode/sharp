/* 派生计算:反向链接 · 图谱数据(全部由 wikilink 推导,单一数据源 = 卡片正文) */
import { Card, GraphEdge, GraphNode } from "./types";
import { extractLinks, nameOf } from "./markdown";

export interface Backlink {
  card: Card;
  rel: string;
}

/** 哪些卡片链接到了 path(反向链接) */
export function backlinksOf(cards: Card[], path: string): Backlink[] {
  const out: Backlink[] = [];
  for (const c of cards) {
    if (c.path === path) continue;
    for (const l of extractLinks(c.body)) {
      if (l.path === path) out.push({ card: c, rel: l.rel });
    }
  }
  return out;
}

export interface GraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

/** 卡片 + 未解析 wikilink → 有向图(边:A 含 [B|rel] 则 A→B) */
export function graphFrom(cards: Card[]): GraphData {
  const nodes: GraphNode[] = cards.map((c) => ({ id: c.path, title: c.title, summary: c.summary }));
  const edges: GraphEdge[] = [];
  const idx = new Map(nodes.map((n, i) => [n.id, i]));
  for (let i = 0; i < cards.length; i++) {
    for (const l of extractLinks(cards[i].body)) {
      let t = idx.get(l.path);
      if (t === undefined) {
        t = nodes.length;
        idx.set(l.path, t);
        nodes.push({
          id: l.path,
          title: nameOf(l.path),
          summary: "路径未解析;点击可创建。",
          unresolved: true,
        });
      }
      if (t !== i) edges.push({ s: i, t, rel: l.rel });
    }
  }
  return { nodes, edges };
}

export function uid(): string {
  return Math.random().toString(36).slice(2, 10);
}

export function pathFor(title: string): string {
  const slug =
    title
      .trim()
      .toLowerCase()
      .replace(/[\s/\\]+/g, "-")
      .replace(/[^\w一-龥-]+/g, "")
      .slice(0, 40) || uid();
  return `.knowledges/notes/${slug}.md`;
}

export function relTime(ts: number): string {
  const d = Math.max(0, Date.now() - ts);
  const m = Math.floor(d / 60000);
  if (m < 1) return "刚刚";
  if (m < 60) return `${m} 分钟前`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h} 小时前`;
  return `${Math.floor(h / 24)} 天前`;
}
