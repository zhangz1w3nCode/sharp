import { forwardRef, useMemo, useState } from "react";
import { Pencil, X } from "lucide-react";
import { useApp } from "../lib/store";
import { Star } from "../components/Icon";
import Markdown from "../components/Markdown";
import EmptyDots from "../components/EmptyDots";
import { backlinksOf, relTime } from "../lib/derive";

/** 知识库 · 卡片网格(筛选 + 收藏) */
export const KbMain = forwardRef<HTMLElement>(function KbMain(_props, ref) {
  const { state, api } = useApp();
  const [filter, setFilter] = useState("");

  const list = useMemo(() => {
    const f = filter.trim().toLowerCase();
    return state.cards.filter((c) => !f || c.title.toLowerCase().includes(f) || c.summary.toLowerCase().includes(f));
  }, [state.cards, filter]);

  return (
    <main ref={ref} className="pane pane-main">
      <div className="m-head">
        <div className="m-title">
          知识库<span className="m-count">· {list.length} 张</span>
        </div>
        <input
          className="m-search"
          placeholder="筛选卡片…"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        />
      </div>
      {state.cards.length === 0 ? (
        <EmptyDots hint="⌘N 新建第一张卡片" />
      ) : list.length === 0 ? (
        <EmptyDots hint="无匹配卡片" />
      ) : (
        <div className="card-grid">
          {list.map((c) => (
            <div
              key={c.id}
              className={`card${state.selectedId === c.id ? " selected" : ""}`}
              onClick={() => api.selectCard(state.selectedId === c.id ? null : c.id)}
            >
              <div className="t">{c.title}</div>
              <div className="s">{c.summary}</div>
              <Star
                on={c.starred}
                onClick={(e) => {
                  e.stopPropagation();
                  api.toggleStar(c.id);
                }}
              />
            </div>
          ))}
        </div>
      )}
    </main>
  );
});

/** 详情面板(读模式)— 摘要 / 内容 / 反向链接 */
export const DetailPanel = forwardRef<HTMLElement>(function DetailPanel(_props, ref) {
  const { state, api } = useApp();
  const card = state.cards.find((c) => c.id === state.selectedId) ?? null;
  const backlinks = useMemo(() => (card ? backlinksOf(state.cards, card.path) : []), [state.cards, card]);

  return (
    <section ref={ref} className={`pane pane-detail${state.detailOpen && card ? "" : " collapsed"}`}>
      {card && (
        <>
          <div className="flex items-center gap-1 -mb-1">
            <button className="tb" style={{ marginLeft: "auto", width: 24, height: 24 }} title="关闭详情 (Esc)" onClick={() => api.selectCard(null)}>
              <X size={13} strokeWidth={1.8} />
            </button>
          </div>
          <div className="d-scroll">
            <div className="flex items-center gap-2">
              <div className="d-title">{card.title}</div>
              <button className="tb" title="编辑" onClick={() => api.openEditor(card.id)}>
                <Pencil size={14} strokeWidth={1.7} />
              </button>
              <Star on={card.starred} size={16} onClick={() => api.toggleStar(card.id)} />
            </div>
            <div>
              <div className="eyebrow" style={{ marginBottom: 7 }}>
                摘要
              </div>
              <div className="d-sum">{card.summary || "（无摘要）"}</div>
            </div>
            <div>
              <div className="eyebrow" style={{ marginBottom: 7 }}>
                内容
              </div>
              <Markdown md={card.body} />
            </div>
            <div>
              <div className="eyebrow" style={{ marginBottom: 4 }}>
                反向链接 · {backlinks.length}
              </div>
              {backlinks.length === 0 ? (
                <div className="empty-dots" style={{ padding: "14px 0" }}>
                  <i />
                  <i />
                  <i />
                </div>
              ) : (
                <div className="d-links">
                  {backlinks.map((b, i) => (
                    <div key={i} className="bl-item" onClick={() => api.selectCard(b.card.id)}>
                      <span className="bl-src">{b.card.title}</span>
                      <span className="bl-rel">· {b.rel}</span>
                      <span className="bl-rel" style={{ marginLeft: "auto" }}>
                        {relTime(b.card.updatedAt)}
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        </>
      )}
    </section>
  );
});
