import { Fragment, useMemo, useState } from "react";
import { ChevronDown, Check, Pencil, Columns2, Link2, ArrowLeft, Trash2 } from "lucide-react";
import { useApp } from "../lib/store";
import { ReviewItem } from "../lib/types";
import { diffLines } from "../lib/diff";
import { esc } from "../lib/markdown";
import EmptyDots from "../components/EmptyDots";

function renderFull(md: string): string {
  return esc(md)
    .replace(/^##\s(.*)$/gm, '<div class="h">$1</div>')
    .replace(/\n/g, "<br>");
}

/** 知识审核 · 待审队列 + 行级 diff */
export default function ReviewView() {
  const { state, api } = useApp();
  const [selId, setSelId] = useState<string | null>(null);
  const [openId, setOpenId] = useState<string | null>(null);
  const [diffId, setDiffId] = useState<string | null>(null);
  const [leaving, setLeaving] = useState<Set<string>>(new Set());

  const diffItem = useMemo(() => state.queue.find((q) => q.id === diffId) ?? null, [state.queue, diffId]);
  const diffRows = useMemo(() => (diffItem ? diffLines(diffItem.current ?? "", diffItem.body) : []), [diffItem]);

  function approve(item: ReviewItem) {
    setLeaving((s) => new Set(s).add(item.id));
    setTimeout(() => {
      api.approve(item.id);
      setLeaving((s) => {
        const n = new Set(s);
        n.delete(item.id);
        return n;
      });
      if (diffId === item.id) setDiffId(null);
    }, 300);
  }

  /* ---------- diff 视图 ---------- */
  if (diffItem) {
    return (
      <main className="pane pane-main" style={{ padding: 16, gap: 16, display: "flex" }}>
        <div className="flex items-center gap-2">
          <div className="m-title" style={{ flex: 1 }}>
            {diffItem.title}
          </div>
          <span className="badge">{diffItem.source}</span>
          <div className="rv-status">
            <span className="dot" />
            待审
          </div>
        </div>
        <div className="flex-1 min-h-0 flex flex-col">
          <div className="diff-headers">
            <div className="diff-col-h">
              <span className="dot muted" />
              当前 · 已通过
            </div>
            <div className="diff-col-h">
              <span className="dot" />
              提议 · 待审
            </div>
          </div>
          <div className="diff-grid flex-1">
            {diffRows.map((r, i) => (
              <Fragment key={i}>
                <div className={`ln ${r.t === "add" ? "empty" : r.t}`}>
                  <span className="sign">{r.t === "del" ? "-" : " "}</span>
                  <span>{r.t === "add" ? "·" : r.l}</span>
                </div>
                <div className={`ln ${r.t === "del" ? "empty" : r.t}`}>
                  <span className="sign">{r.t === "add" ? "+" : " "}</span>
                  <span>{r.t === "del" ? "·" : r.r}</span>
                </div>
              </Fragment>
            ))}
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button className="btn btn-secondary btn-icon" title="返回队列" onClick={() => setDiffId(null)}>
            <ArrowLeft size={15} strokeWidth={1.7} />
          </button>
          <div style={{ marginLeft: "auto", display: "flex", gap: 8 }}>
            <button className="btn btn-secondary btn-icon" title="编辑提议" onClick={() => api.openEditorProposal(diffItem)}>
              <Pencil size={15} strokeWidth={1.7} />
            </button>
            <button className="btn btn-primary btn-icon" title="通过" onClick={() => approve(diffItem)}>
              <Check size={15} strokeWidth={2} />
            </button>
          </div>
        </div>
      </main>
    );
  }

  /* ---------- 队列视图 ---------- */
  return (
    <main className="pane pane-main">
      <div className="m-head">
        <div className="m-title">
          知识审核<span className="m-count">· {state.queue.length} 待审</span>
        </div>
      </div>
      {state.queue.length === 0 ? (
        <EmptyDots hint="队列已清空" />
      ) : (
        <div className="rv-list">
          {state.queue.map((it) => (
            <div
              key={it.id}
              className={`rv-item${selId === it.id ? " sel" : ""}${openId === it.id ? " open" : ""}${leaving.has(it.id) ? " leaving" : ""}`}
              onClick={() => setSelId(selId === it.id ? null : it.id)}
            >
              <div className="rv-row1">
                <div className="rv-t">{it.title}</div>
                {it.type === "new" && (
                  <span
                    className="rv-chev"
                    onClick={(e) => {
                      e.stopPropagation();
                      setOpenId(openId === it.id ? null : it.id);
                    }}
                  >
                    <ChevronDown size={12} strokeWidth={2} />
                  </span>
                )}
                <div className="rv-status">
                  <span className="dot" />
                  待审
                </div>
              </div>
              <div className="rv-s">{it.snippet}</div>
              <div className="rv-row2">
                <span className="badge">{it.source}</span>
                <span className="rv-tag">
                  {it.type === "update" ? <Columns2 size={11} strokeWidth={1.7} /> : <Link2 size={11} strokeWidth={2} />}
                  {it.type === "update" ? "更新" : "新建"}
                </span>
                <div className="rv-acts">
                  {it.type === "update" && (
                    <button
                      className="btn btn-ghost btn-icon-sm btn-icon"
                      title="查看改动"
                      onClick={(e) => {
                        e.stopPropagation();
                        setDiffId(it.id);
                      }}
                    >
                      <Columns2 size={13} strokeWidth={1.7} />
                    </button>
                  )}
                  <button
                    className="btn btn-ghost btn-icon-sm btn-icon"
                    title="编辑提议"
                    onClick={(e) => {
                      e.stopPropagation();
                      api.openEditorProposal(it);
                    }}
                  >
                    <Pencil size={13} strokeWidth={1.7} />
                  </button>
                  <button
                    className="btn btn-ghost btn-icon-sm btn-icon"
                    title="拒绝"
                    onClick={(e) => {
                      e.stopPropagation();
                      api.reject(it.id);
                    }}
                  >
                    <Trash2 size={13} strokeWidth={1.7} />
                  </button>
                  <button
                    className="btn btn-primary btn-icon-sm btn-icon"
                    title="通过"
                    onClick={(e) => {
                      e.stopPropagation();
                      approve(it);
                    }}
                  >
                    <Check size={13} strokeWidth={2} />
                  </button>
                </div>
              </div>
              {it.type === "new" && <div className="rv-full" dangerouslySetInnerHTML={{ __html: renderFull(it.body) }} />}
            </div>
          ))}
        </div>
      )}
    </main>
  );
}
