# TODO — 知识库桌面应用 (Tauri + React + TS)

依据 `fe/` 原型 (prototypes-v2 + base-resource) 实现前端应用。

- [x] 通读 fe/ 全部文件 (prototypes-v2 15 页 + base-resource 令牌/组件/图标/mock)
- [x] 脚手架 app/ (vite + react18 + ts + tailwind v4 + tauri v2 配置)
- [x] 样式层:tokens.css / components.css 移植 + app.css (Tailwind @theme 映射 + 页面专属 CSS)
- [x] 数据层:types / markdown(hl+render+extractLinks) / derive(backlinks+graph) / diff / mock / store(context+reducer+localStorage)
- [x] 通用组件:Icon / Toast / Titlebar / Sidebar / Resizer / EmptyDots / Markdown / CodeEditor(@插链)
- [x] 视图:KbView(网格+详情) / EditorView(编辑+实时预览) / ReviewView(队列+diff) / GraphView(force-directed)
- [x] 浮层:SearchPalette(⌘K) / Sheets(新建/设置/断链创建)
- [x] src-tauri 最小 Rust 工程 (overlay titlebar)
- [x] 验证:npm install + tsc + vite build 通过;cargo check 通过
- [x] 冒烟:headless Chrome 截图(KB/审核/图谱/详情)+ puppeteer 交互测试 6/6 PASS
  (@插链 / sheet提交→审核 / 通过入库 / diff / ⌘K搜索打开 / 编辑器打开)

## 修复过的问题
- extractLinks 丢失 `.knowledges/` 前缀 → 图谱出现伪断链节点
- code 块/inline code 内字面 `[path|rel]` 被误解析为链接 → 渲染前抽占位符
- 原型 markdown.js render() h2 二次转义 bug → 移植时修正
- 图谱模拟循环闭包捕获首次渲染的 cx/cy(容器测量后中心力目标过期)→ 图谱偏左上;改为每帧从 ref 读最新值
- React StrictMode(dev)双挂载:cleanup 取消 rAF 但 id 未归零 → 模拟循环永不启动、图谱空白;cleanup 补 `raf = 0`
- `.graph-wrap svg` 后代选择器泄漏 → 悬浮卡内 lucide 图标被 width/height:100% 放大成巨图标;改直接子代选择器 `.graph-wrap > svg`
- HMR 热替换不重跑 effect,死循环状态残留 → 拖动/设置看似不生效;重启进程后 puppeteer 验证 6/6 PASS(拖动/Node size/Text fade/Repel/图标尺寸/9 节点)

## 后续(需要时再做)
- [ ] 后端持久化(当前 localStorage;Tauri fs/sqlite 插件)
- [ ] 图谱边关系标签渲染
- [ ] 编辑器 split 比例拖拽
- [ ] 暗色模式(当前浅色锁定,与原型一致)
