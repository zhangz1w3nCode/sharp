import { forwardRef } from "react";
import { useApp } from "../lib/store";
import FileTree from "./FileTree";

/** 侧栏 — 文件树全局索引 */
const Sidebar = forwardRef<HTMLElement>(function Sidebar(_props, ref) {
  const { state, api } = useApp();
  const searchDetailCard = state.cards.find((c) => c.id === state.searchDetailId) ?? null;
  return (
    <aside ref={ref} className={`pane pane-side${state.sidebarCollapsed ? " collapsed" : ""}`}>
      {state.view === "kb" && <FileTree key="kb" />}
      {state.view === "search" && (
        <FileTree
          key="search"
          selectedPath={searchDetailCard?.path ?? null}
          onOpenFile={(path) => {
            const c = state.cards.find((x) => x.path === path);
            if (c) api.setSearchDetail(c.id);
            else api.openSheet({ kind: "broken", path });
          }}
        />
      )}
    </aside>
  );
});

export default Sidebar;
