import { useMemo, MouseEvent } from "react";
import { render } from "../lib/markdown";
import { useApp } from "../lib/store";

/**
 * 读模式 markdown 渲染(.md 作用域样式见 app.css)
 * wikilink 点击:存在 → 打开对应卡片;断链 → 创建 sheet
 */
export default function Markdown({ md }: { md: string }) {
  const { state, api } = useApp();
  const paths = useMemo(() => new Set(state.cards.map((c) => c.path)), [state.cards]);
  const html = useMemo(() => render(md, (p) => paths.has(p)), [md, paths]);

  function onClick(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest("a.wl") as HTMLAnchorElement | null;
    if (!a) return;
    e.preventDefault();
    const path = a.dataset.path;
    if (path) api.openCardByPath(path);
  }

  return <div className="md" onClick={onClick} dangerouslySetInnerHTML={{ __html: html }} />;
}
