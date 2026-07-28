/* 品牌符与特殊图标(lucide 之外的部分) */

/** Anthropic 径向 spike 品牌符(fill 继承 currentColor) */
export function Spike({ size = 15, className }: { size?: number; className?: string }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="currentColor"
      className={className}
      style={{ display: "block" }}
      aria-hidden
    >
      <path d="M12 2c.6 4.6 3.4 7.4 8 8-4.6.6-7.4 3.4-8 8-.6-4.6-3.4-7.4-8-8 4.6-.6 7.4-3.4 8-8z" />
    </svg>
  );
}

/** 收藏星 — 珊瑚填充态,对齐 components.css .star */
export function Star({
  on,
  size = 14,
  onClick,
  className = "",
}: {
  on: boolean;
  size?: number;
  onClick?: (e: React.MouseEvent) => void;
  className?: string;
}) {
  return (
    <svg
      className={`star${on ? " on" : ""} ${className}`}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      onClick={onClick}
      aria-hidden
    >
      <path d="M12 3l2.6 5.3 5.8.8-4.2 4.1 1 5.8L12 16.8 6.8 19l1-5.8-4.2-4.1 5.8-.8z" />
    </svg>
  );
}
