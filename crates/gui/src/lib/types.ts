/* 数据模型 — 前端内存态(无后端;持久化走 localStorage) */

export interface Card {
  id: string;
  /** 形如 .knowledges/notes/xxx.md,wikilink 解析的唯一键 */
  path: string;
  title: string;
  summary: string;
  /** raw markdown */
  body: string;
  starred: boolean;
  updatedAt: number;
}

export interface ReviewItem {
  id: string;
  type: "new" | "update";
  /** 来源:大模型 / 手动新建 */
  source: string;
  title: string;
  summary: string;
  snippet: string;
  /** 提议内容(raw markdown) */
  body: string;
  /** update 时的当前已通过版本(用于 diff) */
  current?: string;
  /** update 的目标卡片 path */
  targetPath?: string;
  /** 新建时预定的 path */
  path: string;
  ts: number;
}

export interface GraphNode {
  id: string; // path
  title: string;
  summary: string;
  unresolved?: boolean;
}

export interface GraphEdge {
  s: number;
  t: number;
  rel: string;
}
