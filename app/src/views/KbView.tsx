import { forwardRef, useEffect, useMemo, useRef, useState } from "react";
import { Pencil, X, Check } from "lucide-react";
import { useApp } from "../lib/store";
import Markdown from "../components/Markdown";
import CodeEditor from "../components/CodeEditor";
import EmptyDots from "../components/EmptyDots";
import { backlinksOf } from "../lib/derive";
import { useFade } from "../lib/useFade";

/** 知识库 · 卡片网格(无头部,内容即界面) */
export const KbMain = forwardRef<HTMLElement>(function KbMain(_props, ref) {
  const { state, api } = useApp();
  const list = state.cards;
  const gridRef = useFade<HTMLDivElement>([list.length]);

  return (
    <main ref={ref} className="pane pane-main">
      {state.cards.length === 0 ? (
        <EmptyDots hint="⌘N 新建第一张卡片" />
      ) : (
        <div className="card-grid fade-scroll" ref={gridRef}>
          {list.map((c) => (
            <div
              key={c.id}
              className={`card${state.selectedId === c.id ? " selected" : ""}`}
              onClick={() => api.selectCard(state.selectedId === c.id ? null : c.id)}
            >
              <div className="t">{c.title}</div>
              <div className="s">{c.summary}</div>
            </div>
          ))}
        </div>
      )}
    </main>
  );
});

/** 详情面板 · 编辑态(与读态同处一面板) */
function DetailEdit() {
  const { state, api } = useApp();
  const ed = state.editor;
  const [title, setTitle] = useState(ed?.title ?? "");
  const [summary, setSummary] = useState(ed?.summary ?? "");
  const [body, setBody] = useState(ed?.body ?? "");
  /* 摘要 textarea 高度自适应内容,与读态行高一致(否则 rows 固定会把下方分割线顶下去) */
  const sumRef = useRef<HTMLTextAreaElement>(null);
  useEffect(() => {
    const el = sumRef.current;
    if (el) {
      el.style.height = "auto";
      el.style.height = el.scrollHeight + "px";
    }
  }, [summary]);

  if (!ed) return null;
  const dirty = title !== ed.title || summary !== ed.summary || body !== ed.body;
  const canSubmit = ed.cardId === null ? true : dirty && title.trim().length > 0;

  /* 结构与读态完全镜像:标题行 → 分隔线 → 摘要(导语规格)→ 分隔线 → 内容 */
  return (
    <>
      <div className="flex items-center gap-2 shrink-0">
        <input
          className="title-input"
          placeholder="卡片标题…"
          value={title}
          autoFocus={ed.cardId === null}
          onChange={(e) => setTitle(e.target.value)}
          style={{ flex: 1, minWidth: 0 }}
        />
        <button className="btn btn-ghost btn-icon-sm btn-icon" title="取消 (Esc)" onClick={() => api.closeEditor()}>
          <X size={14} strokeWidth={1.8} />
        </button>
        <button
          className="btn btn-primary btn-icon-sm btn-icon"
          title="确认 → 生成待审"
          disabled={!canSubmit}
          onClick={() => api.submitEditor({ ...ed, title, summary, body })}
        >
          <Check size={14} strokeWidth={2} />
        </button>
      </div>
      <div className="d-div shrink-0" />
      <div className="flex-1 min-h-0 flex flex-col" style={{ gap: 20 }}>
        <textarea ref={sumRef} className="s-input" rows={1} placeholder="一句话摘要…" value={summary} style={{ overflow: "hidden" }} onChange={(e) => setSummary(e.target.value)} />
        <div className="d-sec-div flex-1 min-h-0 flex flex-col">
          <CodeEditor value={body} onChange={setBody} height="flex" />
        </div>
      </div>
    </>
  );
}

/** 详情面板 · 读态(平铺在纸上:摘要 / 内容 / 反向链接) */
function DetailRead() {
  const { state, api } = useApp();
  const card = state.cards.find((c) => c.id === state.selectedId) ?? null;
  const backlinks = useMemo(() => (card ? backlinksOf(state.cards, card.path) : []), [state.cards, card]);
  const bodyRef = useFade<HTMLDivElement>([card?.id]);

  if (!card) return null;
  return (
    <>
      <div className="flex items-center gap-2 shrink-0">
        <div className="d-title">{card.title}</div>
        <button className="btn btn-ghost btn-icon-sm btn-icon" title="编辑" onClick={() => api.openEditor(card.id)}>
          <Pencil size={14} strokeWidth={1.7} />
        </button>
        <button className="btn btn-ghost btn-icon-sm btn-icon" title="关闭详情 (Esc)" onClick={() => api.selectCard(null)}>
          <X size={14} strokeWidth={1.8} />
        </button>
      </div>
      <div className="d-div shrink-0" />
      <div className="d-scroll fade-scroll scroll-thin" ref={bodyRef}>
        <div className="d-sum">{card.summary || "(无摘要)"}</div>
        <div className="d-sec-div">
          <Markdown md={card.body} />
        </div>
      </div>
      {backlinks.length > 0 && (
        <div className="d-links">
          {backlinks.map((b, i) => (
            <div key={i} className="bl-item" onClick={() => api.selectCard(b.card.id)}>
              <span className="d" />
              <span className="bl-src">{b.card.title}</span>
              <span className="chip acc bl-rel">{b.rel}</span>
            </div>
          ))}
        </div>
      )}
    </>
  );
}

/** 详情面板:读/写同处 · 全场最亮的纸 */
export const DetailPanel = forwardRef<HTMLElement>(function DetailPanel(_props, ref) {
  const { state } = useApp();
  const open = state.detailOpen && (state.editor !== null || state.selectedId !== null);
  return (
    <section ref={ref} className={`pane pane-detail${open ? "" : " collapsed"}`} style={{ display: "flex", gap: 14 }}>
      {open && (state.editor ? <DetailEdit key={state.editor.cardId ?? "new-" + (state.editor.originReviewId ?? "")} /> : <DetailRead />)}
    </section>
  );
});
