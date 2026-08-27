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

const WL_RE = /\[\.knowledges\/([^|\]]+)(?:\|([^\]]+))?\]/g;
/* 双括号 + 反引号格式: [[`.knowledges/path`|relation]] */
const WL_RE_DOUBLE = /\[\[`\.knowledges\/([^`]+)`(?:\|([^\]]+))?\]\]/g;

/** 编辑器高亮(暗色面) */
export function hl(md: string): string {
  let out = esc(md);
  out = out.replace(/```[\s\S]*?```/g, (m) => '<span class="cb">' + m + "</span>"); // fence → muted
  out = out.replace(/^#{1,6}\s.*$/gm, (m) => '<span class="h">' + m + "</span>"); // heading → on-dark bold
  out = out.replace(/\*\*([^*]+)\*\*/g, "<b>$1</b>");
  out = out.replace(/`[^`]+`/g, (m) => '<span class="ic">' + m + "</span>"); // inline code → amber
  out = out.replace(/\[\.knowledges[^\]]+\]/g, (m) => '<span class="lk">' + m + "</span>"); // wikilink → coral
  out = out.replace(/\[\[`.knowledges[^\]]+\]\]/g, (m) => '<span class="lk">' + m + "</span>"); // 双括号 wikilink
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
        return '<pre class="md-pre"><code>' + esc(inner) + "</code></pre>";
      }
      let h = esc(part);
      /* 先提取双括号 wikilink [[`.knowledges/path`|rel]] 为占位符,防止反引号被 inline code 提取 */
      const wlDoubles: string[] = [];
      h = h.replace(WL_RE_DOUBLE, (_m, p: string, r?: string) => {
        const path = ".knowledges/" + p;
        const broken = resolve ? !resolve(path) : false;
        const rel = (r || "").trim();
        wlDoubles.push(
          '<a class="md-link' + (broken ? " broken" : "") + '" data-path="' + esc(path) + '">' +
          esc(nameOf(p)) + (rel && !broken ? '<span class="md-rel">' + esc(rel) + "</span>" : "") +
          "</a>"
        );
        return "\u0001" + (wlDoubles.length - 1) + "\u0001";
      });
      /* 先抽出 inline code 为占位符:文档中字面值 `[path|rel]` 不应被解析为链接 */
      const codes: string[] = [];
      h = h.replace(/`([^`]+)`/g, (_m, c: string) => {
        codes.push(c);
        return "\u0000" + (codes.length - 1) + "\u0000";
      });
      h = h.replace(/^### (.*)$/gm, '<h3 class="md-h">$1</h3>');
      h = h.replace(/^## (.*)$/gm, '<h2 class="md-h">$1</h2>');
      h = h.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
      /* wikilink → 珊瑚 chip,关系内联显示;断链弱化 */
      h = h.replace(WL_RE, (_m, p: string, r?: string) => {
        const path = ".knowledges/" + p;
        const broken = resolve ? !resolve(path) : false;
        const rel = (r || "").trim();
        return (
          '<a class="md-link' +
          (broken ? " broken" : "") +
          '" data-path="' +
          esc(path) +
          '">' +
          esc(nameOf(p)) +
          (rel && !broken ? '<span class="md-rel">' + esc(rel) + "</span>" : "") +
          "</a>"
        );
      });
      h = h.replace(/^[-*] (.*)$/gm, "<li>$1</li>");
      h = h.replace(/((?:<li>.*<\/li>\n?)+)/g, "<ul>$1</ul>");
      h = h.replace(/\u0000(\d+)\u0000/g, (_m, i: string) => '<code class="md-ic">' + codes[Number(i)] + "</code>");
      h = h.replace(/\u0001(\d+)\u0001/g, (_m, i: string) => wlDoubles[Number(i)]);
      h = h
        .split(/\n{2,}/)
        .map((b) => (/^\s*<(h2|h3|ul|pre)/.test(b) ? b : '<p class="md-p">' + b.replace(/\n/g, "<br>") + "</p>"))
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
  const noCodeBlock = md.replace(/```[\s\S]*?```/g, "");
  const out: WikiLink[] = [];
  let m: RegExpExecArray | null;
  /* 双括号格式: [[`.knowledges/path`|relation]] — 先提取,避免反引号被 inline code 清理 */
  const reDouble = new RegExp(WL_RE_DOUBLE.source, "g");
  while ((m = reDouble.exec(noCodeBlock))) {
    out.push({ path: ".knowledges/" + m[1], rel: (m[2] || "链接").trim() });
  }
  /* 移除双括号 wikilink 和 inline code 后,提取单括号格式 */
  const clean = noCodeBlock.replace(WL_RE_DOUBLE, "").replace(/`[^`]+`/g, "");
  const reSingle = new RegExp(WL_RE.source, "g");
  while ((m = reSingle.exec(clean))) {
    out.push({ path: ".knowledges/" + m[1], rel: (m[2] || "链接").trim() });
  }
  return out;
}
