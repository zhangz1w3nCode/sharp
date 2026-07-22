import { forwardRef } from "react";
import { useApp, View } from "../lib/store";

const ENTRIES: { view: View; label: string }[] = [
  { view: "kb", label: "知识库" },
  { view: "review", label: "知识审核" },
  { view: "graph", label: "知识图谱" },
];

/** 侧栏 — color-block 选中态(珊瑚圆点),审核带待办计数 */
const Sidebar = forwardRef<HTMLElement>(function Sidebar(_props, ref) {
  const { state, api } = useApp();
  return (
    <aside ref={ref} className={`pane pane-side${state.sidebarCollapsed ? " collapsed" : ""}`}>
      <nav className="flex flex-col gap-0.5 text-[13px] tracking-[0.1px]">
        {ENTRIES.map(({ view, label }) => (
          <div
            key={view}
            className={`entry${state.view === view ? " active" : ""}`}
            onClick={() => api.setView(view)}
          >
            <span className="dot" />
            <span style={{ flex: 1 }}>{label}</span>
          </div>
        ))}
      </nav>
    </aside>
  );
});

export default Sidebar;
