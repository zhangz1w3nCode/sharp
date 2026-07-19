import { forwardRef } from "react";
import { Layers, CheckCheck, Waypoints } from "lucide-react";
import { useApp, View } from "../lib/store";

const ENTRIES: { view: View; label: string; icon: typeof Layers }[] = [
  { view: "kb", label: "知识库", icon: Layers },
  { view: "review", label: "知识审核", icon: CheckCheck },
  { view: "graph", label: "知识图谱", icon: Waypoints },
];

/** 侧栏 — color-block 选中态(珊瑚圆点),审核带待办计数 */
const Sidebar = forwardRef<HTMLElement>(function Sidebar(_props, ref) {
  const { state, api } = useApp();
  return (
    <aside ref={ref} className={`pane pane-side${state.sidebarCollapsed ? " collapsed" : ""}`}>
      <nav className="flex flex-col gap-0.5 text-[13px] tracking-[0.1px]">
        {ENTRIES.map(({ view, label, icon: Icon }) => (
          <div
            key={view}
            className={`entry${state.view === view ? " active" : ""}`}
            onClick={() => api.setView(view)}
          >
            <span className="dot" />
            <Icon size={14} strokeWidth={1.7} style={{ flex: "0 0 auto", opacity: 0.85 }} />
            <span style={{ flex: 1 }}>{label}</span>
            {view === "review" && state.queue.length > 0 && (
              <span style={{ fontSize: 11, color: "var(--muted-soft)" }}>{state.queue.length}</span>
            )}
          </div>
        ))}
      </nav>
    </aside>
  );
});

export default Sidebar;
