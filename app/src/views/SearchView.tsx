import { useEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent } from "react";
import { Search, ArrowLeft, Pencil } from "lucide-react";
import { useApp } from "../lib/store";
import { esc } from "../lib/markdown";
import { backlinksOf } from "../lib/derive";
import { useFade } from "../lib/useFade";
import Markdown from "../components/Markdown";
import { Card } from "../lib/types";
import { DetailEdit } from "./KbView";

type Hit = { card: Card; kind: "title" | "summary" | "body"; snippet: string };
type Mode = "center" | "browsing" | "detail";

function mark(text: string, q: string): string {
  if (!q) return esc(text);
  const i = text.toLowerCase().indexOf(q.toLowerCase());
  if (i < 0) return esc(text);
  return esc(text.slice(0, i)) + "<mark>" + esc(text.slice(i, i + q.length)) + "</mark>" + esc(text.slice(i + q.length));
}

/** 全文搜索:标题 > 摘要 > 正文;正文命中取匹配处前后 ~30 字符上下文 */
function searchCards(cards: Card[], query: string): Hit[] {
  const ql = query.toLowerCase();
  const hits: Hit[] = [];
  for (const c of cards) {
    if (c.title.toLowerCase().includes(ql)) {
      hits.push({ card: c, kind: "title", snippet: c.summary });
    } else if (c.summary.toLowerCase().includes(ql)) {
      hits.push({ card: c, kind: "summary", snippet: c.summary });
    } else {
      const flat = c.body.replace(/\s+/g, " ");
      const i = flat.toLowerCase().indexOf(ql);
      if (i >= 0) {
        const start = Math.max(0, i - 30);
        const end = Math.min(flat.length, i + query.length + 30);
        const snippet = (start > 0 ? "…" : "") + flat.slice(start, end) + (end < flat.length ? "…" : "");
        hits.push({ card: c, kind: "body", snippet });
      }
    }
  }
  const order: Record<Hit["kind"], number> = { title: 0, summary: 1, body: 2 };
  return hits.sort((a, b) => order[a.kind] - order[b.kind] || b.card.updatedAt - a.card.updatedAt);
}

export default function SearchView() {
  const { state, api } = useApp();
  const [mode, setMode] = useState<Mode>("center");
  const [q, setQ] = useState("");
  const [sel, setSel] = useState(-1);
  const [searchedQ, setSearchedQ] = useState<string | null>(null);
  const [prevMode, setPrevMode] = useState<Mode>("center");
  const [typing, setTyping] = useState(false);

  const inpRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const gridRef = useFade<HTMLDivElement>([searchedQ]);
  const bodyRef = useFade<HTMLDivElement>([state.searchDetailId]);

  const suggestions = useMemo(() => (q.trim() ? searchCards(state.cards, q.trim()).slice(0, 8) : []), [q, state.cards]);
  const gridResults = useMemo(() => (searchedQ ? searchCards(state.cards, searchedQ) : []), [searchedQ, state.cards]);
  const detailCard = useMemo(() => (state.searchDetailId ? state.cards.find((c) => c.id === state.searchDetailId) ?? null : null), [state.searchDetailId, state.cards]);
  const backlinks = useMemo(() => (detailCard ? backlinksOf(state.cards, detailCard.path) : []), [state.cards, detailCard]);

  useEffect(() => { inpRef.current?.focus(); }, []);
  useEffect(() => { if (mode !== "detail") inpRef.current?.focus(); }, [mode]);
  useEffect(() => { if (sel >= 0) listRef.current?.children[sel]?.scrollIntoView({ block: "nearest" }); }, [sel]);
  useEffect(() => { if (state.searchDetailId && mode !== "detail") { setPrevMode(mode); setMode("detail"); } }, [state.searchDetailId]);

  function open(hit: Hit) {
    setPrevMode(mode);
    api.setSearchDetail(hit.card.id);
    setMode("detail");
    setSel(-1);
    setTyping(false);
  }

  function back() {
    setMode(prevMode);
    api.setSearchDetail(null);
    setTyping(false);
  }

  function onKeyDown(e: ReactKeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter") {
      e.preventDefault();
      if (sel >= 0 && suggestions[sel]) {
        open(suggestions[sel]);
      } else if (q.trim()) {
        setSearchedQ(q.trim());
        setTyping(false);
        setMode("browsing");
        setSel(-1);
      }
    } else if (e.key === "ArrowDown") {
      if (suggestions.length) {
        e.preventDefault();
        setSel((p) => Math.min(p + 1, suggestions.length - 1));
      }
    } else if (e.key === "ArrowUp") {
      if (suggestions.length) {
        e.preventDefault();
        setSel((p) => Math.max(p - 1, -1));
      }
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      if (q || suggestions.length) {
        setQ("");
        setSel(-1);
        setTyping(false);
        if (mode === "browsing") {
          setMode("center");
          setSearchedQ(null);
        }
      }
    }
  }

  /* Esc in detail mode: document 级监听(detail 无输入框焦点) */
  useEffect(() => {
    if (mode !== "detail") return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") back();
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [mode]);

  /* 编辑器激活时(新建/编辑):在主面板渲染 DetailEdit */
  if (state.editor) {
    return (
      <main className="pane pane-main search-detail">
        <DetailEdit key={state.editor.cardId ?? "new"} />
      </main>
    );
  }
  /* ============ DETAIL MODE: 卡片详情在主面板中间区域 ============ */
  if (mode === "detail" && detailCard) {
    return (
      <main className="pane pane-main search-detail">
        <div className="flex items-center gap-2 shrink-0">
          <button className="btn btn-ghost btn-icon-sm btn-icon" title="返回搜索" onClick={back}>
            <ArrowLeft size={15} strokeWidth={1.7} />
          </button>
          <div className="d-title">{detailCard.title}</div>
          <button className="btn btn-ghost btn-icon-sm btn-icon" title="编辑" onClick={() => api.openEditor(detailCard.id, true)}>
            <Pencil size={14} strokeWidth={1.7} />
          </button>
        </div>
        <div className="d-div shrink-0" />
        <div className="d-scroll fade-scroll scroll-thin" ref={bodyRef}>
          <div className="d-sum">{detailCard.summary || "(无摘要)"}</div>
          <div className="d-sec-div">
            <Markdown md={detailCard.body} onLinkClick={(path) => {
              const c = state.cards.find((x) => x.path === path);
              if (c) api.setSearchDetail(c.id);
              else api.openSheet({ kind: "broken", path });
            }} />
          </div>
        </div>
        {backlinks.length > 0 && (
          <div className="d-links">
            {backlinks.map((b, i) => (
              <div key={i} className="bl-item" onClick={() => api.setSearchDetail(b.card.id)}>
                <span className="d" />
                <span className="bl-src">{b.card.title}</span>
                <span className="chip acc bl-rel">{b.rel}</span>
              </div>
            ))}
          </div>
        )}
      </main>
    );
  }

  /* ============ BROWSING MODE: 搜索框移至顶部 + 卡片网格 ============ */
  if (mode === "browsing") {
    return (
      <main className="pane pane-main search-browsing">
        <div className="search-top-bar">
          <Search size={18} strokeWidth={1.6} className="search-top-icon" />
          <input
            ref={inpRef}
            className="search-top-input"
            type="text"
            value={q}
            onChange={(e) => { setQ(e.target.value); setSel(-1); setTyping(true); }}
            onKeyDown={onKeyDown}
          />
          {suggestions.length > 0 && typing && (
            <div className="search-dropdown" ref={listRef}>
              {suggestions.map((h, i) => (
                <div
                  key={h.card.id}
                  className={`sd-item${i === sel ? " sel" : ""}`}
                  onMouseEnter={() => setSel(i)}
                  onClick={() => open(h)}
                >
                  <div className="sd-t" dangerouslySetInnerHTML={{ __html: mark(h.card.title, q.trim()) }} />
                  <div className="sd-s" dangerouslySetInnerHTML={{ __html: mark(h.snippet, q.trim()) }} />
                </div>
              ))}
            </div>
          )}
        </div>
        {gridResults.length === 0 ? (
          <div className="empty-dots" style={{ padding: "40px 0" }}>
            <i /><i /><i />
          </div>
        ) : (
          <div className="card-grid fade-scroll" ref={gridRef}>
            {gridResults.map((h) => (
              <div
                key={h.card.id}
                className="card"
                onClick={() => { setPrevMode("browsing"); api.setSearchDetail(h.card.id); setMode("detail"); }}
              >
                <div className="t" dangerouslySetInnerHTML={{ __html: mark(h.card.title, searchedQ ?? "") }} />
                <div className="s" dangerouslySetInnerHTML={{ __html: mark(h.card.summary, searchedQ ?? "") }} />
              </div>
            ))}
          </div>
        )}
      </main>
    );
  }

  /* ============ CENTER MODE: 空态 + 实时下拉建议 ============ */
  return (
    <div className="pane pane-main search-page">
      <div className="search-title">What's up</div>
      <div className="search-field">
        <Search className="search-field-icon" size={20} strokeWidth={1.6} />
        <input
          ref={inpRef}
          className="search-input"
          type="text"
          value={q}
          onChange={(e) => { setQ(e.target.value); setSel(-1); setTyping(true); }}
          onKeyDown={onKeyDown}
        />
        {suggestions.length > 0 && typing && (
          <div className="search-dropdown" ref={listRef}>
            {suggestions.map((h, i) => (
              <div
                key={h.card.id}
                className={`sd-item${i === sel ? " sel" : ""}`}
                onMouseEnter={() => setSel(i)}
                onClick={() => open(h)}
              >
                <div className="sd-t" dangerouslySetInnerHTML={{ __html: mark(h.card.title, q.trim()) }} />
                <div className="sd-s" dangerouslySetInnerHTML={{ __html: mark(h.snippet, q.trim()) }} />
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
