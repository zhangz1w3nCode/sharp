import { forwardRef } from "react";
import { useApp } from "../lib/store";

/** 侧栏 — 页面级操作区,内容待定,当前留空 */
const Sidebar = forwardRef<HTMLElement>(function Sidebar(_props, ref) {
  const { state } = useApp();
  return (
    <aside ref={ref} className={`pane pane-side${state.sidebarCollapsed ? " collapsed" : ""}`} />
  );
});

export default Sidebar;
