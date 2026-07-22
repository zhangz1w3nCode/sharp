import { useMemo, useState } from "react";
import { Check, Pencil, ArrowLeft } from "lucide-react";
import { useApp } from "../lib/store";
import { diffLines } from "../lib/diff";
import EmptyDots from "../components/EmptyDots";

/**
 * 知识审核 · 点击卡片即看 diff(GitHub 式双栏)
 * 卡片上没有任何按钮;动作全部收敛在 diff 页:返回 / 编辑 / 拒绝 / 通过
 */
export default function ReviewView() {
  const { state, api } = useApp();
  const [diffId, setDiffId] = useState<string | null>(null);
  const [leaving, setLeaving] = useState(false);

  const item = useMemo(() => state.queue.find((q) => q.id === diffId) ?? null, [state.queue, diffId]);
  const rows = useMemo(() => (item ? diffLines(item.current ?? "", item.body) : []), [item]);

  function approve() {
    if (!item) return;
    setLeaving(true);
    setTimeout(() => {
      api.approve(item.id);
      setLeaving(false);
      setDiffId(null);
    }, 300);
  }
  /* ---------- diff 页(update:左右对照 · new:左空右全绿) ---------- */
  if (item) {
    return (
      <main className={`pane pane-main${leaving ? " leaving" : ""}`} style={{ padding: 16, gap: 16, display: "flex" }}>
        <div className="flex items-center gap-2 shrink-0">
          <div className="d-title" style={{ fontSize: 18 }}>
            {item.title}
          </div>
          <span className="badge">{item.source}</span>
          <span className="chip acc">{item.type === "update" ? "更新" : "新建"}</span>
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
            {rows.map((r, i) => (
              <div key={i} style={{ display: "contents" }}>
                <div className={`ln ${r.t === "add" ? "empty" : r.t}`}>
                  <span className="sign">{r.t === "del" ? "-" : " "}</span>
                  <span>{r.t === "add" ? "·" : r.l}</span>
                </div>
                <div className={`ln ${r.t === "del" ? "empty" : r.t}`}>
                  <span className="sign">{r.t === "add" ? "+" : " "}</span>
                  <span>{r.t === "del" ? "·" : r.r}</span>
                </div>
              </div>
            ))}
          </div>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <button className="btn btn-secondary btn-icon" title="返回队列" onClick={() => setDiffId(null)}>
            <ArrowLeft size={15} strokeWidth={1.7} />
          </button>
          <div style={{ marginLeft: "auto", display: "flex", gap: 8 }}>
            <button className="btn btn-secondary btn-icon" title="编辑提议" onClick={() => api.openEditorProposal(item)}>
              <Pencil size={15} strokeWidth={1.7} />
            </button>
            <button className="btn btn-primary btn-icon" title="通过" onClick={approve}>
              <Check size={15} strokeWidth={2} />
            </button>
          </div>
        </div>
      </main>
    );
  }

  /* ---------- 队列(纯卡片,无任何按钮) ---------- */
  return (
    <main className="pane pane-main">
      {state.queue.length === 0 ? (
        <EmptyDots hint="队列已清空" />
      ) : (
        <div className="rv-list">
          {state.queue.map((it) => (
            <div key={it.id} className="rv-item" onClick={() => setDiffId(it.id)}>
              <div className="rv-t">{it.title}</div>
              <div className="rv-s">{it.snippet}</div>
              <div className="rv-row2">
                <span className="badge">{it.source}</span>
              </div>
            </div>
          ))}
        </div>
      )}
    </main>
  );
}
