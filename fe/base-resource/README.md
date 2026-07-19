# base-resource · 设计系统分层

Claude design 风格的单一源。原型与未来 Tauri/React 应用均引用此处。

```
base-resource/
  theme/        DESIGN.md(①②) · tokens.ts(②·给 React)
  foundation/   type-scale · motion · grid    (③ — 当前折叠进 component/tokens.css,按需拆)
  component/    tokens.css(②) · components.css(④·原型) · react/(④·工程,待建) · README.md
  icons/        icons.svg(sprite)             (③·图标)
  js/           ui.js · markdown.js            (横切·原型交互)
  data/         mock.js                        (横切·数据)
  template/     page.html                     (⑥·新页样板)
  tailwind/      tailwind.config.js           (横切·工具链,脱 CDN 后)
```

## 已建(可用)
- `theme/DESIGN.md` — 设计规格(①②)
- `component/tokens.css` — 令牌(色/字/圆角/距/层级),21 页引用
- `component/components.css` `component/README.md` — 原型组件类,21 页引用
- `js/ui.js` — toast · makeResizer · windowChrome(交通灯+标题栏拖拽)
- `icons/icons.svg` — 图标 sprite
- `data/mock.js` — 卡片/图谱/审核 mock fixtures
- `theme/tokens.ts` — TS 令牌(给 React)

## 待建/待迁
- `js/markdown.js` — hl() + renderPreview()(4 页重复,待抽)
- `template/page.html` — 新页样板
- `tailwind/tailwind.config.js` — 共享 config(脱 CDN 后)
- `component/react/` — Button.tsx / Card.tsx …(真实 React 应用,需构建环境)
- `foundation/` — type/motion/grid 拆分(当前在 tokens.css)

## 复用方式
- CSS: `<link rel="stylesheet" href="../../base-resource/component/tokens.css">`
- JS: `<script src="../../base-resource/js/ui.js"></script>` → `window.UI.toast(...)`
- 图标: `<svg class="ico" width="16" height="16"><use href="../../base-resource/icons/icons.svg#search"/></svg>`
- 数据: `<script src="../../base-resource/data/mock.js"></script>` → `window.MOCK.CARDS`
