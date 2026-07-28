/* =========================================================================
   store.tsx — 应用状态(context + useReducer + localStorage 持久化)
   无后端:审核通过即写内存态并落盘;刷新后从 localStorage 恢复
   ========================================================================= */
import { createContext, useContext, useEffect, useMemo, useReducer, ReactNode } from "react";
import { Card, ReviewItem } from "./types";
import { pathFor, uid } from "./derive";
import { mockCards, mockQueue } from "../data/mock";

export type View = "kb" | "review" | "graph" | "search" | "kanban";
export type SheetState = { kind: "settings" } | { kind: "broken"; path: string } | null;

export interface EditorState {
  cardId: string | null; // null = 新建
  path: string;
  title: string;
  summary: string;
  body: string;
  /** 编辑某待审项的提议:提交后替换原待审项 */
  originReviewId?: string;
}

interface State {
  view: View;
  cards: Card[];
  queue: ReviewItem[];
  selectedId: string | null;
  detailOpen: boolean;
  sidebarCollapsed: boolean;
  sheet: SheetState;
  editor: EditorState | null;
  toast: { msg: string; key: number } | null;
  kbRoot: string;
  searchDetailId: string | null;
  _searchSidebarRestore: boolean;
}

type Action =
  | { type: "view"; view: View }
  | { type: "select"; id: string | null }
  | { type: "toggleStar"; id: string }
  | { type: "toggleSidebar" }
  | { type: "sheet"; sheet: SheetState }
  | { type: "editor"; editor: EditorState | null; keepView?: boolean }
  | { type: "toast"; msg: string }
  | { type: "approve"; id: string }
  | { type: "reject"; id: string }
  | { type: "enqueue"; item: ReviewItem }
  | { type: "upsertCard"; card: Card }
  | { type: "searchDetail"; id: string | null };

const LS_KEY = "kb-state-v1";

function load(): { cards: Card[]; queue: ReviewItem[] } {
  try {
    const raw = localStorage.getItem(LS_KEY);
    if (raw) {
      const p = JSON.parse(raw);
      if (Array.isArray(p.cards) && Array.isArray(p.queue)) return { cards: p.cards, queue: p.queue };
    }
  } catch {
    /* 损坏则回退 mock */
  }
  return { cards: mockCards, queue: mockQueue };
}

function initView(): View {
  const h = typeof location !== "undefined" ? location.hash : "";
  if (h.startsWith("#/review")) return "review";
  if (h.startsWith("#/graph")) return "graph";
  if (h.startsWith("#/search")) return "search";
  if (h.startsWith("#/kanban")) return "kanban";
  return "kb";
}

function initState(): State {
  const { cards, queue } = load();
  /* hash 深链:#/kb/<path> 直接打开卡片详情 */
  let selectedId: string | null = null;
  const m = typeof location !== "undefined" ? location.hash.match(/^#\/kb\/(\..+)$/) : null;
  if (m) {
    const c = cards.find((x) => x.path === decodeURIComponent(m[1]));
    if (c) selectedId = c.id;
  }
  const initialView = initView();
  return {
    view: initialView,
    cards,
    queue,
    selectedId,
    detailOpen: selectedId !== null,
    sidebarCollapsed: initialView === "search",
    sheet: null,
    editor: null,
    toast: null,
    kbRoot: ".knowledges",
    searchDetailId: null,
    _searchSidebarRestore: false,
  };
}

function reducer(s: State, a: Action): State {
  switch (a.type) {
    case "view":
      if (a.view === "search" && s.view !== "search")
        return { ...s, view: a.view, _searchSidebarRestore: s.sidebarCollapsed, sidebarCollapsed: true };
      if (s.view === "search" && a.view !== "search")
        return { ...s, view: a.view, sidebarCollapsed: s._searchSidebarRestore };
      return { ...s, view: a.view };
    case "select":
      /* 选中其他卡片 = 退出编辑(同 Esc),保证网格选中与详情内容永远一致 */
      return { ...s, selectedId: a.id, detailOpen: a.id !== null, editor: null };
    case "toggleStar":
      return { ...s, cards: s.cards.map((c) => (c.id === a.id ? { ...c, starred: !c.starred } : c)) };
    case "toggleSidebar":
      if (s.view === "search") {
        const next = !s.sidebarCollapsed;
        return { ...s, sidebarCollapsed: next, _searchSidebarRestore: next };
      }
      return { ...s, sidebarCollapsed: !s.sidebarCollapsed };
    case "searchDetail":
      return { ...s, searchDetailId: a.id };
    case "sheet":
      return { ...s, sheet: a.sheet };
    case "editor":
      /* keepView: 搜索页编辑留在当前视图;默认跳知识库展开面板 */
      if (a.editor)
        return {
          ...s,
          editor: a.editor,
          view: a.keepView ? s.view : "kb",
          detailOpen: true,
          selectedId: a.editor.cardId ?? null,
        };
      return { ...s, editor: null };
    case "toast":
      return { ...s, toast: { msg: a.msg, key: Date.now() } };
    case "enqueue":
      return { ...s, queue: [{ ...a.item }, ...s.queue] };
    case "upsertCard": {
      const i = s.cards.findIndex((c) => c.id === a.card.id);
      const cards = i >= 0 ? s.cards.map((c) => (c.id === a.card.id ? a.card : c)) : [a.card, ...s.cards];
      return { ...s, cards };
    }
    case "approve": {
      const item = s.queue.find((q) => q.id === a.id);
      if (!item) return s;
      let cards = s.cards;
      if (item.type === "new") {
        const card: Card = {
          id: uid(),
          path: item.path,
          title: item.title,
          summary: item.summary,
          body: item.body,
          starred: false,
          updatedAt: Date.now(),
        };
        cards = [card, ...cards];
      } else if (item.targetPath) {
        cards = cards.map((c) =>
          c.path === item.targetPath
            ? { ...c, title: item.title, summary: item.summary, body: item.body, updatedAt: Date.now() }
            : c,
        );
      }
      return { ...s, cards, queue: s.queue.filter((q) => q.id !== a.id) };
    }
    case "reject":
      return { ...s, queue: s.queue.filter((q) => q.id !== a.id) };
    default:
      return s;
  }
}

export interface Api {
  setView: (v: View) => void;
  selectCard: (id: string | null) => void;
  openCardByPath: (path: string) => void;
  toggleStar: (id: string) => void;
  toggleSidebar: () => void;
  setSearchDetail: (id: string | null) => void;
  openSheet: (sheet: SheetState) => void;
  openEditor: (cardId: string | null, keepView?: boolean) => void;
  openEditorProposal: (item: ReviewItem) => void;
  closeEditor: () => void;
  submitEditor: (e: EditorState) => void;
  approve: (id: string) => void;
  reject: (id: string) => void;
  createFromBroken: (path: string) => void;
  toast: (msg: string) => void;
}

const Ctx = createContext<{ state: State; api: Api } | null>(null);

export function AppProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, undefined, initState);

  useEffect(() => {
    try {
      localStorage.setItem(LS_KEY, JSON.stringify({ cards: state.cards, queue: state.queue }));
    } catch {
      /* 存储不可用时静默 */
    }
  }, [state.cards, state.queue]);

  const api = useMemo<Api>(() => {
    const toast = (msg: string) => dispatch({ type: "toast", msg });
    return {
      setView: (view) => dispatch({ type: "view", view }),
      selectCard: (id) => dispatch({ type: "select", id }),
      openCardByPath: (path) => {
        const c = state.cards.find((x) => x.path === path);
        if (c) {
          dispatch({ type: "view", view: "kb" });
          dispatch({ type: "select", id: c.id });
        } else {
          dispatch({ type: "sheet", sheet: { kind: "broken", path } });
        }
      },
      toggleStar: (id) => dispatch({ type: "toggleStar", id }),
      toggleSidebar: () => dispatch({ type: "toggleSidebar" }),
      setSearchDetail: (id) => dispatch({ type: "searchDetail", id }),
      openSheet: (sheet) => dispatch({ type: "sheet", sheet }),
      openEditor: (cardId, keepView) => {
        if (cardId === null) {
          dispatch({
            type: "editor",
            editor: { cardId: null, path: "", title: "", summary: "", body: "## 新笔记\n\n开始书写…" },
            keepView,
          });
          return;
        }
        const c = state.cards.find((x) => x.id === cardId);
        if (!c) return;
        dispatch({
          type: "editor",
          editor: { cardId: c.id, path: c.path, title: c.title, summary: c.summary, body: c.body },
          keepView,
        });
      },
      openEditorProposal: (item) => {
        const target = item.targetPath ? state.cards.find((c) => c.path === item.targetPath) : null;
        dispatch({
          type: "editor",
          editor: {
            cardId: target?.id ?? null,
            path: item.path,
            title: item.title,
            summary: item.summary,
            body: item.body,
            originReviewId: item.id,
          },
          keepView: true,
        });
      },
      closeEditor: () => dispatch({ type: "editor", editor: null }),
      submitEditor: (e) => {
        const isNew = e.cardId === null;
        const path = isNew ? pathFor(e.title) : e.path;
        const current = isNew ? undefined : state.cards.find((c) => c.id === e.cardId)?.body;
        const item: ReviewItem = {
          id: uid(),
          type: isNew ? "new" : "update",
          source: "手动新建",
          title: e.title.trim() || "未命名卡片",
          summary: e.summary.trim(),
          snippet: e.summary.trim() || e.body.replace(/[#`\[\]]/g, "").slice(0, 60),
          body: e.body,
          current,
          targetPath: isNew ? undefined : path,
          path,
          ts: Date.now(),
        };
        if (e.originReviewId) dispatch({ type: "reject", id: e.originReviewId });
        dispatch({ type: "enqueue", item });
        dispatch({ type: "editor", editor: null });
        dispatch({ type: "view", view: "review" });
        toast("已生成待审版本 → 知识审核");
      },
      approve: (id) => {
        dispatch({ type: "approve", id });
        toast("已通过 → 知识库");
      },
      reject: (id) => {
        dispatch({ type: "reject", id });
        toast("已移除该待审项");
      },
      createFromBroken: (path) => {
        const name = path.match(/([^/]+)\.md$/)?.[1] || "未命名卡片";
        const item: ReviewItem = {
          id: uid(),
          type: "new",
          source: "手动新建",
          title: name,
          summary: "",
          snippet: "由断链创建,内容待补充。",
          body: `## ${name}\n\n(待补充)`,
          path,
          ts: Date.now(),
        };
        dispatch({ type: "enqueue", item });
        dispatch({ type: "sheet", sheet: null });
        toast("已生成待审 → 知识审核");
      },
      toast,
    };
  }, [state.cards]);

  /* 视图 ↔ hash 同步(刷新保持所在视图) */
  useEffect(() => {
    const sel = state.view === "kb" && state.selectedId ? state.cards.find((c) => c.id === state.selectedId) : null;
    const h = "#/" + state.view + (sel ? "/" + encodeURIComponent(sel.path) : "");
    if (location.hash !== h) history.replaceState(null, "", h);
  }, [state.view, state.selectedId, state.cards]);

  const value = useMemo(() => ({ state, api }), [state, api]);
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useApp() {
  const v = useContext(Ctx);
  if (!v) throw new Error("useApp must be used within AppProvider");
  return v;
}
