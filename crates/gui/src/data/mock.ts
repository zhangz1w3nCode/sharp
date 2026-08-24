/* =========================================================================
   mock.ts — 移植自 fe/base-resource/data/mock.js + 原型内联 fixtures
   差异:卡片 body 统一为 raw markdown(原型 home.html 内联 HTML 仅用于展示,
   真实应用以 md 为单一数据源,编辑器/读模式/图谱均由此推导)
   ========================================================================= */
import { Card, ReviewItem } from "../lib/types";

const now = Date.now();
const min = (n: number) => now - n * 60000;

export const mockCards: Card[] = [
  {
    id: "c-rsc",
    path: ".knowledges/notes/rsc.md",
    title: "React Server Components 速记",
    summary: "RSC 在服务端渲染、不 hydrate;与 Client Components 边界靠 'use client'。",
    starred: true,
    updatedAt: min(2),
    body: `## 边界

服务端组件不能 \`useState\` / \`useEffect\`。客户端组件以 \`'use client'\` 声明。相关见 [.knowledges/notes/tauri-vibrancy.md|相关]。

\`\`\`
// 边界靠指令
'use client'
export function Button(){…}
\`\`\``,
  },
  {
    id: "c-tauri",
    path: ".knowledges/notes/tauri-vibrancy.md",
    title: "Tauri v2 窗口 vibrancy",
    summary: "macOS 下窗口毛玻璃的实现要点与 NSVisualEffectView 兼容配置。",
    starred: false,
    updatedAt: min(8),
    body: `## 实现

通过 \`windowEffects\` 配置 vibrancy,需 NSVisualEffectView 兼容。相关见 [.knowledges/notes/macos-hig.md|引用] 与 [.knowledges/notes/wikilink.md|相关]。

\`\`\`
// tauri.conf.json
"windowEffects": { "effects": ["sidebar"], "state": "active" }
\`\`\`

## 要点

- 配置 NSVisualEffectView + NSHUDWindowMaterial
- 断链如 [.knowledges/notes/unknown.md|引用] 显示虚线,点击可创建
- 字体回退见 [.knowledges/notes/sfpro.md|扩展]`,
  },
  {
    id: "c-tw",
    path: ".knowledges/notes/tailwind-theme.md",
    title: "Tailwind v4 @theme 令牌",
    summary: "@theme 定义 --color-* 即生成工具类,替代 tailwind.config。",
    starred: true,
    updatedAt: min(21),
    body: `## 用法

在 CSS 中以 \`@theme\` 声明令牌,工具类自动生成,替代 tailwind.config。

\`\`\`
@theme {
  --color-canvas: #faf9f5;
  --color-primary: #cc785c;
}
\`\`\`

相关见 [.knowledges/notes/no-dividers.md|相关]。`,
  },
  {
    id: "c-graph",
    path: ".knowledges/notes/graph-layout.md",
    title: "知识图谱 force-directed 布局",
    summary: "d3-force / vis-network;节点按入度放大,边有向。",
    starred: false,
    updatedAt: min(31),
    body: `## 规则

- 节点按入度(in-degree)放大
- 边有向:A 含 [B|…] 则 A→B
- 断链节点虚线浅色

依赖 [.knowledges/notes/wikilink.md|依赖]。`,
  },
  {
    id: "c-wl",
    path: ".knowledges/notes/wikilink.md",
    title: "Wikilink 解析 [path|relation]",
    summary: "正则提取 path 与 relation;断链虚线 + 创建入口。",
    starred: false,
    updatedAt: min(40),
    body: `## 格式

\`[.knowledges/notes/x.md|关系]\` · relation 为自由文本。读模式只显文件名。

\`\`\`
// 提取
const m = s.match(/\\[([^|\\]]+)(?:\\|([^\\]]+))?\\]/);
\`\`\`

相关见 [.knowledges/notes/tauri-vibrancy.md|相关]。`,
  },
  {
    id: "c-hig",
    path: ".knowledges/notes/macos-hig.md",
    title: "macOS HIG 三原则",
    summary: "Clarity(清晰)/ Deference(克制)/ Depth(层次)。",
    starred: true,
    updatedAt: min(66),
    body: `## 三原则

- Clarity — 内容优先,UI 克制
- Deference — 控件不抢戏
- Depth — 层级靠模糊与表面色,非阴影

被 [.knowledges/notes/no-dividers.md|引用] 进一步发挥。`,
  },
  {
    id: "c-sfpro",
    path: ".knowledges/notes/sfpro.md",
    title: "SF Pro 字体回退栈",
    summary: "-apple-system 在 macOS 解析为真 SF Pro;跨平台用 Inter 近似。",
    starred: false,
    updatedAt: min(90),
    body: `## 回退

\`\`\`
font-family: 'Inter',
  -apple-system, BlinkMacSystemFont,
  'Segoe UI', sans-serif;
\`\`\``,
  },
  {
    id: "c-nd",
    path: ".knowledges/notes/no-dividers.md",
    title: "极简设计:去分隔线",
    summary: "表面色反差 + 间隙分层,不用 hairline,更克制高级。",
    starred: false,
    updatedAt: min(120),
    body: `## 手法

- box 隔离:色调反差 + 圆角
- 无描边、无分割线
- 留白承担分层

见 [.knowledges/notes/macos-hig.md|引用]。`,
  },
];

export const mockQueue: ReviewItem[] = [
  {
    id: "q-1",
    type: "new",
    source: "大模型",
    title: "Agentic 工作流速记",
    summary: "Agent = 模型 + 工具 + 循环;终止条件比提示词更关键。",
    snippet: "Agent = 模型 + 工具 + 循环;终止条件比提示词更关键。",
    ts: min(2),
    path: ".knowledges/notes/agentic-loop.md",
    body: `## Agentic 工作流

Agent = 模型 + 工具 + 循环;终止条件比提示词更关键。

- 工具调用是唯一的副作用出口
- 每轮都要可观测(trace)
- 终止条件:任务完成 / 预算耗尽 / 人工接管`,
  },
  {
    id: "q-2",
    type: "update",
    source: "大模型",
    title: "Tauri v2 窗口 vibrancy",
    summary: "macOS 下窗口毛玻璃的实现要点,补充 NSHUDWindowMaterial 兼容。",
    snippet: "更新 vibrancy 配置说明,补充 NSHUDWindowMaterial 兼容。",
    ts: min(8),
    path: ".knowledges/notes/tauri-vibrancy.md",
    targetPath: ".knowledges/notes/tauri-vibrancy.md",
    current: `## 实现

通过 \`windowEffects\` 配置 vibrancy,需 NSVisualEffectView 兼容。

\`\`\`
// tauri.conf.json
"windowEffects": { "effects": ["sidebar"] }
\`\`\`

## 要点

- 配置 NSVisualEffectView
- 字体回退见 SF Pro`,
    body: `## 实现

通过 \`windowEffects\` 配置 vibrancy,需 NSVisualEffectView + NSHUDWindowMaterial 兼容。

\`\`\`
// tauri.conf.json
"windowEffects": { "effects": ["sidebar"] }
\`\`\`

## 要点

- 配置 NSVisualEffectView + NSHUDWindowMaterial
- 字体回退见 [.knowledges/notes/sfpro.md|扩展]
- 断链处理见 [.knowledges/notes/wikilink.md|相关]`,
  },
  {
    id: "q-3",
    type: "new",
    source: "手动新建",
    title: "极简设计:表面即分层",
    summary: "表面色反差(canvas→card→muted)+ 间隙分层,不用 hairline。",
    snippet: "表面色反差(canvas→card→muted)+ 间隙分层,不用 hairline。",
    ts: min(15),
    path: ".knowledges/notes/surface-depth.md",
    body: `## 表面即分层

表面色反差(canvas→card→muted)+ 间隙分层,不用 hairline。

- box 隔离:色调反差 + 圆角
- 无描边、无分割线
- 与 [.knowledges/notes/no-dividers.md|相关] 互补`,
  },
  {
    id: "q-4",
    type: "update",
    source: "大模型",
    title: "Wikilink 解析 [path|relation]",
    summary: "slug 概念改为相对 path;关系为自由文本。",
    snippet: "slug 概念改为相对 path;关系为自由文本。",
    ts: min(23),
    path: ".knowledges/notes/wikilink.md",
    targetPath: ".knowledges/notes/wikilink.md",
    current: `## 格式

\`[slug|ref]\` · ref 为预定义枚举。

\`\`\`
const m = s.match(/\\[([^|\\]]+)\\|([^\\]]+)\\]/);
\`\`\``,
    body: `## 格式

\`[.knowledges/notes/x.md|关系]\` · relation 为自由文本。读模式只显文件名。

\`\`\`
// 提取
const m = s.match(/\\[([^|\\]]+)(?:\\|([^\\]]+))?\\]/);
\`\`\``,
  },
  {
    id: "q-5",
    type: "new",
    source: "大模型",
    title: "搜索即命令面板",
    summary: "⌘K 唤起,标题 + 摘要双字段匹配,键盘全程可达。",
    snippet: "⌘K 唤起,标题 + 摘要双字段匹配,键盘全程可达。",
    ts: min(31),
    path: ".knowledges/notes/cmd-palette.md",
    body: `## 命令面板

- ⌘K 唤起,Esc 关闭
- 标题 + 摘要双字段匹配
- ↑↓ 选择,Enter 打开,全程不离键盘`,
  },
];
