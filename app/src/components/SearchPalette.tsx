import { useEffect, useMemo, useRef, useState } from "react";
import { Search, X } from "lucide-react";
import { useApp } from "../lib/store";
import { esc } from "../lib/markdown";

function mark(text: string, q: string): string {
  if (!q) return esc(text);
  const i = text.toLowerCase().indexOf(q.toLowerCase());
  if (i < 0) return esc(text);
  return esc(text.slice(0, i)) + "<mark>" + esc(text.slice(i, i + q.length)) + "</mark>" + esc(text.slice(i + q.length));
}

/** ⌘K 搜索面板 — 顶部下拉,标题+摘要匹配,键盘全程可达 */
export default function SearchPalette() {
  const { state, api } = useApp();
  const [q, setQ] = useState("");
  const [sel, setSel] = useState(0);
  const inpRef = useRef<HTMLInputElement>(null);

  const results = useMemo(() => {
    const ql = q.trim().toLowerCase();
    if (!ql) return [];
    return state.cards.filter((c) => c.title.toLowerCase().includes(ql) || c.summary.toLowerCase().includes(ql)).slice(0, 8);
  }, [q, state.cards]);

  useEffect(() => {
    if (state.searchOpen) {
      setQ("");
      setSel(0);
      setTimeout(() => inpRef.current?.focus(), 60);
    }
  }, [state.searchOpen]);

  useEffect(() => setSel(0), [q]);

  if (!state.searchOpen) return null;

  function close() {
    api.setSearch(false);
  }
  function open(i: number) {
    const c = results[i];
    if (!c) return;
    api.setSearch(false);
    api.setView("kb");
    api.selectCard(c.id);
  }

  return (
    <>
      <div className="overlay show" style={{ background: "transparent", backdropFilter: "none", WebkitBackdropFilter: "none" }} onClick={close} />
      <div className="search-panel show">
        <div className="qrow">
          <Search className="si" size={13} strokeWidth={1.6} />
          <input
            ref={inpRef}
            className="qinp"
            placeholder="搜索卡片…"
            value={q}
            onChange={(e) => setQ(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") close();
              else if (e.key === "ArrowDown") {
                e.preventDefault();
                if (results.length) setSel((s) => (s + 1) % results.length);
              } else if (e.key === "ArrowUp") {
                e.preventDefault();
                if (results.length) setSel((s) => (s - 1 + results.length) % results.length);
              } else if (e.key === "Enter") {
                e.preventDefault();
                open(sel);
              }
            }}
          />
          <span className="qx" onClick={close}>
            <X size={11} strokeWidth={1.8} />
          </span>
        </div>
        <div className="results">
          {q.trim() === "" ? (
            <div className="qh">输入关键词搜索卡片…</div>
          ) : results.length === 0 ? (
            <div className="empty-dots" style={{ padding: "28px 0" }}>
              <i />
              <i />
              <i />
            </div>
          ) : (
            results.map((c, i) => (
              <div key={c.id} className={`r${i === sel ? " sel" : ""}`} onMouseEnter={() => setSel(i)} onClick={() => open(i)}>
                <span className="t" dangerouslySetInnerHTML={{ __html: mark(c.title, q.trim()) }} />
                <span className="s" dangerouslySetInnerHTML={{ __html: mark(c.summary, q.trim()) }} />
              </div>
            ))
          )}
        </div>
      </div>
    </>
  );
}
