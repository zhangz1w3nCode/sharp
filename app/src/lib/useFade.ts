import { useEffect, useRef } from "react";

/**
 * attachFade — 滚动渐隐遮罩(移植自知识库页 v2)
 * 只在对应方向还有内容时 fade,滚到底/到顶自动消失,避免文字被硬切。
 * deps 变化时重新计算(如列表内容更新)。
 */
export function useFade<T extends HTMLElement>(deps: unknown[] = []) {
  const ref = useRef<T | null>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const upd = () => {
      const top = el.scrollTop > 8;
      const bottom = el.scrollHeight - el.clientHeight - el.scrollTop > 8;
      const m =
        "linear-gradient(to bottom," +
        (top ? "transparent" : "#000") +
        " 0,#000 22px,#000 calc(100% - 22px)," +
        (bottom ? "transparent" : "#000") +
        " 100%)";
      el.style.webkitMaskImage = m;
      el.style.maskImage = m;
    };
    el.addEventListener("scroll", upd);
    window.addEventListener("resize", upd);
    const ro = new ResizeObserver(upd);
    ro.observe(el);
    upd();
    return () => {
      el.removeEventListener("scroll", upd);
      window.removeEventListener("resize", upd);
      ro.disconnect();
      el.style.webkitMaskImage = "";
      el.style.maskImage = "";
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);

  return ref;
}
