import { useState } from "react";
import { X, Check, Eye, EyeOff } from "lucide-react";
import { useApp } from "../lib/store";
import CodeEditor from "../components/CodeEditor";
import Markdown from "../components/Markdown";

/**
 * 编辑器视图(原型 detail/edit.html 的补全)
 * 左:serif 标题 + 摘要 + 暗色 md 编辑器;右:可开关的实时预览(原型未做的分屏)
 * 确认 → 生成待审版本(新建/更新)→ 知识审核
 */
export default function EditorView() {
  const { state, api } = useApp();
  const ed = state.editor;
  const [title, setTitle] = useState(ed?.title ?? "");
  const [summary, setSummary] = useState(ed?.summary ?? "");
  const [body, setBody] = useState(ed?.body ?? "");
  const [preview, setPreview] = useState(true);

  if (!ed) return null;
  const dirty = title !== ed.title || summary !== ed.summary || body !== ed.body;
  const canSubmit = title.trim().length > 0 && (ed.cardId === null ? true : dirty);

  return (
    <div className="editor-pane">
      <main className="pane pane-main editor-box">
        <div className="flex items-center gap-2">
          <input
            className="title-input"
            placeholder="卡片标题"
            value={title}
            autoFocus={ed.cardId === null}
            onChange={(e) => setTitle(e.target.value)}
            style={{ flex: 1 }}
          />
          <button className="btn btn-ghost btn-icon" title={preview ? "隐藏预览" : "显示预览"} onClick={() => setPreview((p) => !p)}>
            {preview ? <EyeOff size={15} strokeWidth={1.7} /> : <Eye size={15} strokeWidth={1.7} />}
          </button>
          <button className="btn btn-ghost btn-icon" title="取消 (Esc)" onClick={() => api.closeEditor()}>
            <X size={15} strokeWidth={1.8} />
          </button>
          <button
            className="btn btn-primary btn-icon"
            title="确认 → 生成待审"
            disabled={!canSubmit}
            onClick={() => api.submitEditor({ ...ed, title, summary, body })}
          >
            <Check size={15} strokeWidth={2} />
          </button>
        </div>
        <div>
          <div className="eyebrow" style={{ marginBottom: 8 }}>
            概述
          </div>
          <textarea className="ed-summary" rows={2} placeholder="一句话概述" value={summary} onChange={(e) => setSummary(e.target.value)} />
        </div>
        <div className="flex flex-col flex-1 min-h-0" style={{ gap: 8 }}>
          <div className="eyebrow">内容 · raw markdown · @ 引用</div>
          <CodeEditor value={body} onChange={setBody} height="flex" />
        </div>
      </main>
      <section className={`pane editor-preview${preview ? "" : " collapsed"}`}>
        <div className="eyebrow" style={{ padding: "2px 4px 10px" }}>
          预览
        </div>
        <div className="d-scroll">
          <div className="d-title">{title || "未命名卡片"}</div>
          {summary.trim() && <div className="d-sum">{summary}</div>}
          <Markdown md={body} />
        </div>
      </section>
    </div>
  );
}
