import { useRef, RefObject, MouseEvent } from "react";

/** 可拖拽分隔条(间隙即分隔,coral tint hover)— 移植 ui.js makeResizer */
export default function Resizer({
  pane,
  min,
  max,
}: {
  pane: RefObject<HTMLElement | null>;
  min: number;
  max: number;
}) {
  const drag = useRef<{ x: number; w: number } | null>(null);

  function onDown(e: MouseEvent) {
    const el = pane.current;
    if (!el) return;
    e.preventDefault();
    drag.current = { x: e.clientX, w: el.offsetWidth };
    document.body.style.userSelect = "none";

    const mv = (ev: globalThis.MouseEvent) => {
      if (!drag.current || !pane.current) return;
      const w = Math.max(min, Math.min(max, drag.current.w + (ev.clientX - drag.current.x)));
      pane.current.style.width = w + "px";
    };
    const up = () => {
      drag.current = null;
      document.body.style.userSelect = "";
      document.removeEventListener("mousemove", mv);
      document.removeEventListener("mouseup", up);
    };
    document.addEventListener("mousemove", mv);
    document.addEventListener("mouseup", up);
  }

  return <div className="resizer" onMouseDown={onDown} />;
}
