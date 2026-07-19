/** 三点空状态 — 无文字的极简占位(对齐 components.css .empty-dots) */
export default function EmptyDots({ hint }: { hint?: string }) {
  return (
    <div className="empty-wrap">
      <div className="empty-dots">
        <i />
        <i />
        <i />
      </div>
      {hint ? <div className="empty-hint">{hint}</div> : null}
    </div>
  );
}
