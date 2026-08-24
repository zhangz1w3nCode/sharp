import { useMemo, useRef, useState, KeyboardEvent, ClipboardEvent } from "react";
import { hl } from "../lib/markdown";
import { useApp } from "../lib/store";

/**
 * 暗色 raw-markdown 编辑器(textarea + 高亮 pre 叠层)
 * — 移植 detail/edit.html:
 *   @ → 两步插链(选卡 → 输关系);Backspace/Delete 在 wikilink 边界 = 整块删
 */
export default function CodeEditor({
  value,
  onChange,
  height = 340,
  placeholder,
  autoFocus,
}: {
  value: string;
  onChange: (v: string) => void;
  height?: number | string;
  placeholder?: string;
  autoFocus?: boolean;
}) {
  const { state } = useApp();
  const taRef = useRef<HTMLTextAreaElement>(null);
  const preRef = useRef<HTMLPreElement>(null);
  const [focus, setFocus] = useState(false);
  // @ 弹层:0 关 · 1 选卡 · 2 输关系
  const [step, setStep] = useState<0 | 1 | 2>(0);
  const [sel, setSel] = useState(0);
  const [rel, setRel] = useState("");
  const [pos, setPos] = useState({ top: 0, left: 0 });
  const atPos = useRef<number | null>(null);
  const picked = useRef<string | null>(null);

  const highlighted = useMemo(() => hl(value) + "\n", [value]);
  const items = state.cards;

  function placePopup() {
    const r = taRef.current?.getBoundingClientRect();
    if (!r) return;
    const top = Math.min(r.bottom + 4, window.innerHeight - 220);
    setPos({ top, left: r.left });
  }

  function replaceAt(text: string) {
    const ta = taRef.current;
    if (!ta) return;
    const a = atPos.current ?? ta.selectionStart - 1;
    const v = ta.value;
    const next = v.slice(0, a) + text + v.slice(a + 1);
    onChange(next);
    requestAnimationFrame(() => {
      ta.selectionStart = ta.selectionEnd = a + text.length;
      ta.focus();
    });
  }

  function pick(i: number) {
    picked.current = items[i]?.path ?? null;
    setStep(2);
    setRel("");
    placePopup();
  }

  function commitRel() {
    if (picked.current) replaceAt("[" + picked.current + (rel.trim() ? "|" + rel.trim() : "") + "]");
    setStep(0);
  }

  function onInput(v: string) {
    onChange(v);
    const ta = taRef.current;
    if (!ta) return;
    const c = ta.selectionStart;
    if (v[c - 1] === "@") {
      atPos.current = c - 1;
      setSel(0);
      setStep(1);
      placePopup();
    } else if (step !== 0) {
      setStep(0);
    }
  }

  function onKeyDown(e: KeyboardEvent<HTMLTextAreaElement>) {
    const ta = taRef.current;
    if (!ta) return;
    // wikilink 整块删除(弹层关闭时)
    if ((e.key === "Backspace" || e.key === "Delete") && ta.selectionStart === ta.selectionEnd && step === 0) {
      const c = ta.selectionStart;
      const v = ta.value;
      if (e.key === "Backspace") {
        const m = v.slice(0, c).match(/\[\.knowledges[^\]]*\]$/);
        if (m) {
          e.preventDefault();
          const s = c - m[0].length;
          onChange(v.slice(0, s) + v.slice(c));
          requestAnimationFrame(() => (ta.selectionStart = ta.selectionEnd = s));
          return;
        }
      } else {
        const m = v.slice(c).match(/^\[\.knowledges[^\]]*\]/);
        if (m) {
          e.preventDefault();
          onChange(v.slice(0, c) + v.slice(c + m[0].length));
          return;
        }
      }
    }
    if (step === 1) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSel((s) => (s + 1) % items.length);
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSel((s) => (s - 1 + items.length) % items.length);
      } else if (e.key === "Enter") {
        e.preventDefault();
        pick(sel);
      } else if (e.key === "Escape") {
        e.preventDefault();
        replaceAt("");
        setStep(0);
      }
    } else if (step === 2 && e.key === "Escape") {
      e.preventDefault();
      replaceAt("");
      setStep(0);
    }
  }

  function onPaste(_e: ClipboardEvent) {
    /* 默认行为即可,保留入口以便后续做 md 规范化 */
  }

  return (
    <div
      className={`code-editor${focus ? " focus" : ""}`}
      style={height === "flex" ? { position: "relative", flex: 1, minHeight: 0 } : { height, position: "relative" }}
    >
      <pre ref={preRef} aria-hidden dangerouslySetInnerHTML={{ __html: highlighted }} />
      <textarea
        ref={taRef}
        value={value}
        spellCheck={false}
        placeholder={placeholder}
        autoFocus={autoFocus}
        onChange={(e) => onInput(e.target.value)}
        onKeyDown={onKeyDown}
        onPaste={onPaste}
        onScroll={(e) => {
          if (preRef.current) {
            preRef.current.scrollTop = (e.target as HTMLTextAreaElement).scrollTop;
            preRef.current.scrollLeft = (e.target as HTMLTextAreaElement).scrollLeft;
          }
        }}
        onFocus={() => setFocus(true)}
        onBlur={() => setFocus(false)}
      />
      {step === 1 && (
        <div className="ac" style={{ top: pos.top, left: pos.left }}>
          <div className="ac-h">@ 插入链接 · 选目标卡</div>
          {items.map((c, i) => (
            <div
              key={c.id}
              className={`ac-item${i === sel ? " sel" : ""}`}
              onMouseDown={(e) => {
                e.preventDefault();
                pick(i);
              }}
            >
              <span className="d" />
              {c.title}
              <span className="ac-path">{c.path}</span>
            </div>
          ))}
          {items.length === 0 && <div className="ac-h">暂无卡片</div>}
        </div>
      )}
      {step === 2 && (
        <div className="ac" style={{ top: pos.top, left: pos.left }}>
          <div className="ac-h">输入关系(自由,如:引用/相关/扩展/依赖…)</div>
          <div className="ac-foot">
            <input
              className="ac-input"
              autoFocus
              value={rel}
              placeholder="关系(可留空 = [path])"
              onChange={(e) => setRel(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  commitRel();
                } else if (e.key === "Escape") {
                  e.preventDefault();
                  replaceAt("");
                  setStep(0);
                }
              }}
            />
            <button className="btn btn-primary" style={{ height: 32, padding: "0 12px" }} onClick={commitRel}>
              插入
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
