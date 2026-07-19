/* =========================================================================
   markdown.ts — 移植自 fe/base-resource/js/markdown.js
   hl()      → 暗色 code-window 编辑器源码高亮(对齐 components.css .code-editor)
   render()  → 奶油面读模式语义 HTML(.wl 珊瑚内链,断链 .broken)
   extractLinks() → 提取 [.knowledges/path|relation] 用于双链/图谱
   修复:原型 render() 中 h2 二次转义的 bug 已修正
   ========================================================================= */

export function esc(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

export function nameOf(path: string): string {
  const m = path.match(/([^/]+)\.md$/);
  return m ? m[1] : path;
}

const WL_RE = /\[\.knowledges\/([^\]|]+)(?:\|([^\]]+))?\]/g;

/** 编辑器高亮(暗色面) */
export function hl(md: string): string {
  let out = esc(md);
  out = out.replace(/```[\s\S]*?```/g, (m) => '<span class="cb">' + m + "</span>"); // fence → muted
  out = out.replace(/^#{1,6}\s.*$/gm, (m) => '<span class="h">' + m + "</span>"); // heading → on-dark bold
  out = out.replace(/\*\*([^*]+)\*\*/g, "<b>$1</b>");
  out = out.replace(/`[^`]+`/g, (m) => '<span class="ic">' + m + "</span>"); // inline code → amber
  out = out.replace(/\[\.knowledges[^\]]+\]/g, (m) => '<span class="lk">' + m + "</span>"); // wikilink → coral
  out = out.replace(/^([-*])\s/gm, '<span class="lm">$1</span> '); // list marker → muted
  return out;
}

/**
 * 读模式渲染。
 * resolve(path) 返回是否存在对应卡片;不存在 → 断链样式(.wl.broken)。
 */
export function render(md: string, resolve?: (path: string) => boolean): string {
  const parts = md.split(/(```[\s\S]*?```)/g);
  return parts
    .map((part) => {
      if (/^```[\s\S]*```$/.test(part)) {
        const inner = part.replace(/^```\w*\n?/, "").replace(/```$/, "");
        return "<pre>" + esc(inner) + "</pre>";
      }
      let h = esc(part);
      /* 先抽出 inline code 为占位符:文档中字面值 `[path|rel]` 不应被解析为链接 */
      const codes: string[] = [];
      h = h.replace(/`([^`]+)`/g, (_m, c: string) => {
        codes.push(c);
        return "\u0000" + (codes.length - 1) + "\u0000";
      });
      h = h.replace(/^### (.*)$/gm, "<h3>$1</h3>");
      h = h.replace(/^## (.*)$/gm, "<h2>$1</h2>");
      h = h.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
      h = h.replace(WL_RE, (_m, p: string, r?: string) => {
        const path = ".knowledges/" + p;
        const broken = resolve ? !resolve(path) : false;
        return (
          '<a class="wl' +
          (broken ? " broken" : "") +
          '" data-path="' +
          esc(path) +
          '" data-rel="' +
          esc((r || "链接").trim()) +
          '">' +
          esc(nameOf(p)) +
          "</a>"
        );
      });
      h = h.replace(/^[-*] (.*)$/gm, "<li>$1</li>");
      h = h.replace(/((?:<li>.*<\/li>\n?)+)/g, "<ul>$1</ul>");
      h = h.replace(/\u0000(\d+)\u0000/g, (_m, i: string) => "<code>" + codes[Number(i)] + "</code>");
      h = h
        .split(/\n{2,}/)
        .map((b) => (/^\s*<(h2|h3|ul|pre)/.test(b) ? b : "<p>" + b.replace(/\n/g, "<br>") + "</p>"))
        .join("\n");
      return h;
    })
    .join("\n");
}

export interface WikiLink {
  path: string;
  rel: string;
}

/** 提取正文中全部 wikilink(双链 / 图谱 / 反向链接的数据源)。code 块与 inline code 中的字面值不算链接。 */
export function extractLinks(md: string): WikiLink[] {
  const clean = md.replace(/```[\s\S]*?```/g, "").replace(/`[^`]+`/g, "");
  const out: WikiLink[] = [];
  const re = new RegExp(WL_RE.source, "g");
  let m: RegExpExecArray | null;
  while ((m = re.exec(clean))) {
    out.push({ path: ".knowledges/" + m[1], rel: (m[2] || "链接").trim() });
  }
  return out;
}
