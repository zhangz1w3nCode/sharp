import { useEffect, useMemo, useRef, useState, KeyboardEvent } from "react";
import { Search } from "lucide-react";
import { useApp } from "../lib/store";
import { esc } from "../lib/markdown";
import { Card } from "../lib/types";

type Hit = { card: Card; kind: "title" | "summary" | "body"; snippet: string };

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
  const [q, setQ] = useState("");
  const [submitted, setSubmitted] = useState<string | null>(null);
  const [sel, setSel] = useState(-1);
  const inpRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const results = useMemo(() => (submitted ? searchCards(state.cards, submitted) : []), [submitted, state.cards]);

  useEffect(() => {
    inpRef.current?.focus();
  }, []);

  useEffect(() => {
    if (sel >= 0) listRef.current?.children[sel]?.scrollIntoView({ block: "nearest" });
  }, [sel]);

  function open(hit: Hit) {
    api.selectCard(hit.card.id);
  }

  function onKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter") {
      e.preventDefault();
      if (sel >= 0 && results[sel]) open(results[sel]);
      else if (q.trim()) {
        setSubmitted(q.trim());
        setSel(-1);
      }
    } else if (e.key === "ArrowDown") {
      if (results.length) {
        e.preventDefault();
        const next = Math.min(sel + 1, results.length - 1);
        setSel(next);
        if (state.detailOpen) api.selectCard(results[next].card.id);
      }
    } else if (e.key === "ArrowUp") {
      if (results.length) {
        e.preventDefault();
        const next = Math.max(sel - 1, -1);
        setSel(next);
        if (state.detailOpen && next >= 0) api.selectCard(results[next].card.id);
      }
    } else if (e.key === "Escape") {
      /* 详情打开时交给全局 Esc 先关详情;否则清空查询回空态 */
      if (state.detailOpen) return;
      if (q || submitted) {
        e.preventDefault();
        e.stopPropagation();
        setQ("");
        setSubmitted(null);
        setSel(-1);
      }
    }
  }

  return (
    <div className={`pane pane-main search-page${submitted !== null ? " has-results" : ""}`}>
      <div className="search-title">What's up</div>
      <div className="search-field">
        <Search className="search-field-icon" size={20} strokeWidth={1.6} />
        <input
          ref={inpRef}
          className="search-input"
          type="text"
          value={q}
          onChange={(e) => setQ(e.target.value)}
          onKeyDown={onKeyDown}
        />
      </div>
      {submitted !== null && (
        <div className="search-results" ref={listRef}>
          {results.length === 0 ? (
            <div className="empty-dots" style={{ padding: "40px 0" }}>
              <i />
              <i />
              <i />
            </div>
          ) : (
            results.map((h, i) => (
              <div
                key={h.card.id}
                className={`sr-item${i === sel ? " sel" : ""}`}
                onMouseEnter={() => setSel(i)}
                onClick={() => open(h)}
              >
                <div className="sr-t" dangerouslySetInnerHTML={{ __html: mark(h.card.title, submitted) }} />
                {h.snippet && <div className="sr-s" dangerouslySetInnerHTML={{ __html: mark(h.snippet, submitted) }} />}
                <div className="sr-p">{h.card.path}</div>
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
}
