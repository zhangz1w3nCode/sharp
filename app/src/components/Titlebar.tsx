import { PanelLeft, Search, Settings, Plus } from "lucide-react";
import { useApp } from "../lib/store";

const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const isMac = typeof navigator !== "undefined" && /Mac/i.test(navigator.platform);

/** 无缝标题栏:品牌 + 拖拽区 + 工具簇(图标优先,无文字) */
export default function Titlebar() {
  const { api } = useApp();
  return (
    <header className="titlebar" data-tauri-drag-region style={isTauri && isMac ? { paddingLeft: 76 } : undefined}>
      <div style={{ flex: 1 }} data-tauri-drag-region />
      <div className="flex items-center gap-1.5 shrink-0">
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
