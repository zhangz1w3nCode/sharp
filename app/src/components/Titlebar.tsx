import { PanelLeft, Search, Settings, Plus } from "lucide-react";
import { useApp, View } from "../lib/store";

const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const isMac = typeof navigator !== "undefined" && /Mac/i.test(navigator.platform);

const NAV: { view: View; label: string }[] = [
  { view: "kb", label: "知识库" },
  { view: "review", label: "知识审核" },
  { view: "graph", label: "知识图谱" },
  { view: "search", label: "搜索" },
];

/** 无缝标题栏:居中分段导航 + 拖拽区 + 工具簇(图标优先,无文字) */
export default function Titlebar() {
  const { state, api } = useApp();
  return (
    <header className="titlebar" data-tauri-drag-region style={isTauri && isMac ? { paddingLeft: 76 } : undefined}>
      <nav className="seg seg-center">
        {NAV.map(({ view, label }) => (
          <div
            key={view}
            className={`seg-item${state.view === view ? " active" : ""}`}
            onClick={() => api.setView(view)}
          >
            {label}
          </div>
        ))}
      </nav>
      <div className="flex items-center gap-1.5 shrink-0" style={{ marginLeft: "auto" }}>
        <button className="tb" title="折叠侧栏 ⌘B" onClick={() => api.toggleSidebar()}>
          <PanelLeft size={16} strokeWidth={1.6} />
        </button>
        <button className="tb" title="搜索 ⌘K" onClick={() => api.setSearch(true)}>
          <Search size={16} strokeWidth={1.6} />
        </button>
        <button className="tb" title="设置" onClick={() => api.openSheet({ kind: "settings" })}>
          <Settings size={16} strokeWidth={1.6} />
        </button>
        <button
          className="btn btn-primary btn-icon"
          style={{ width: 32, height: 32 }}
          title="新建卡片 ⌘N"
          onClick={() => api.openEditor(null)}
        >
          <Plus size={15} strokeWidth={2} />
        </button>
      </div>
    </header>
  );
}
