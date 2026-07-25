import { useEffect, useRef } from "react";
import { AppProvider, useApp } from "./lib/store";
import Titlebar from "./components/Titlebar";
import Sidebar from "./components/Sidebar";
import Resizer from "./components/Resizer";
import Toast from "./components/Toast";
import SearchPalette from "./components/SearchPalette";
import Sheets from "./components/Sheets";
import { KbMain, DetailPanel } from "./views/KbView";
import ReviewView from "./views/ReviewView";
import GraphView from "./views/GraphView";

function Shell() {
  const { state, api } = useApp();
  const sideRef = useRef<HTMLElement>(null);
  const detailRef = useRef<HTMLElement>(null);

  /* 全局快捷键:⌘K 搜索 · ⌘N 新建 · ⌘B 侧栏 · ⌘1/2/3 视图 · Esc 逐层关闭 */
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key.toLowerCase() === "k") {
        e.preventDefault();
        api.setSearch(!state.searchOpen);
      } else if (mod && e.key.toLowerCase() === "n") {
        e.preventDefault();
        api.openEditor(null);
      } else if (mod && e.key.toLowerCase() === "b") {
        e.preventDefault();
        api.toggleSidebar();
      } else if (mod && e.key === "1") {
        e.preventDefault();
        api.setView("kb");
      } else if (mod && e.key === "2") {
        e.preventDefault();
        api.setView("review");
      } else if (mod && e.key === "3") {
        e.preventDefault();
        api.setView("graph");
      } else if (e.key === "Escape") {
        if (state.searchOpen) api.setSearch(false);
        else if (state.sheet) api.openSheet(null);
        else if (state.editor) api.closeEditor();
        else if (state.detailOpen) api.selectCard(null);
      }
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [state, api]);

  return (
    <div className="app">
      <Titlebar />
      <div className="app-body">
        <Sidebar ref={sideRef} />
        {!state.sidebarCollapsed && <Resizer pane={sideRef} min={180} max={380} />}
        {state.view === "kb" && <KbMain />}
        {state.view === "review" && <ReviewView />}
        {state.view === "graph" && <GraphView />}
        {state.view === "kb" && (
          <>
            {state.detailOpen && <Resizer pane={detailRef} min={300} max={1100} />}
            <DetailPanel ref={detailRef} />
          </>
        )}
      </div>
      <SearchPalette />
      <Sheets />
      <Toast />
    </div>
  );
}

export default function App() {
  return (
    <AppProvider>
      <Shell />
    </AppProvider>
  );
}
