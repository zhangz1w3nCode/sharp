import { forwardRef } from "react";
import { useApp } from "../lib/store";
import FileTree from "./FileTree";

/** 侧栏 — 文件树全局索引 */
const Sidebar = forwardRef<HTMLElement>(function Sidebar(_props, ref) {
  const { state } = useApp();
  return (
    <aside ref={ref} className={`pane pane-side${state.sidebarCollapsed ? " collapsed" : ""}`}>
      {state.view === "kb" && <FileTree />}
    </aside>
  );
});

export default Sidebar;
