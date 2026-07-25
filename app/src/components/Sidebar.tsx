import { forwardRef } from "react";
import { useApp } from "../lib/store";

/** 侧栏 — 页面级操作区,内容待定,当前留空 */
const Sidebar = forwardRef<HTMLElement>(function Sidebar(_props, ref) {
  const { state } = useApp();
  /* 搜索视图强制折叠(不动 sidebarCollapsed 持久态,切回其他视图自动恢复) */
  const collapsed = state.sidebarCollapsed || state.view === "search";
  return (
    <aside ref={ref} className={`pane pane-side${collapsed ? " collapsed" : ""}`} />
  );
});

export default Sidebar;
