import { X } from "lucide-react";
import { useApp } from "../lib/store";
import { Spike } from "./Icon";

/** macOS sheet(顶部滑入 + 暖色模糊背景)— 设置 / 断链创建(新建卡片已收拢进详情面板) */
export default function Sheets() {
  const { state, api } = useApp();
  const sheet = state.sheet;

  if (!sheet) return null;
  const close = () => api.openSheet(null);

  return (
    <div
      className="overlay show"
      onClick={(e) => {
        if (e.target === e.currentTarget) close();
      }}
    >
      {sheet.kind === "settings" && (
        <div className="sheet" style={{ display: "flex" }}>
          <div className="flex items-center gap-2">
            <div className="sheet-title" style={{ flex: 1 }}>
              设置
            </div>
            <div className="sheet-x" onClick={close}>
              <X size={14} strokeWidth={1.8} />
            </div>
          </div>
          <div className="set-sec">
            <div className="set-sec-t">外观</div>
            <div className="set-row">
              <span>主题</span>
              <span className="g-toggle on toggle-locked" onClick={() => api.toast("浅色锁定(暗色为派生,暂不开放)")}>
                <span className="sw" />
                浅色
              </span>
            </div>
          </div>
          <div className="set-sec">
            <div className="set-sec-t">Wikilink 格式</div>
            <div className="set-row" style={{ justifyContent: "flex-start", gap: 6 }}>
              <span className="md-inline">[path|relation]</span>
              <span style={{ color: "var(--muted)" }}>或</span>
              <span className="md-inline">[path]</span>
            </div>
          </div>
          <div className="set-sec">
            <div className="set-sec-t">关于</div>
            <div className="set-row" style={{ justifyContent: "flex-start", gap: 8 }}>
              <span className="mark" style={{ display: "inline-flex" }}>
                <Spike size={14} />
              </span>
              <span>知识库桌面应用</span>
              <span style={{ color: "var(--muted-soft)", fontSize: 12 }}>v0.1 · 暖奶油编辑风</span>
            </div>
          </div>
          <div className="sheet-foot">
            <button className="btn btn-secondary" onClick={close}>
              完成
            </button>
          </div>
        </div>
      )}

      {sheet.kind === "broken" && (
        <div className="sheet" style={{ display: "flex" }}>
          <div className="flex items-center gap-2">
            <div className="sheet-title" style={{ flex: 1 }}>
              创建卡片
            </div>
            <div className="sheet-x" onClick={close}>
              <X size={14} strokeWidth={1.8} />
            </div>
          </div>
          <div className="hint">以下路径未解析,是否以此创建卡片?</div>
          <div className="path-box">{sheet.path}</div>
          <div className="sheet-foot">
            <button className="btn btn-secondary" onClick={close}>
              取消
            </button>
            <button className="btn btn-primary" onClick={() => api.createFromBroken(sheet.path)}>
              确认
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
