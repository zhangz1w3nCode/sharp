# Sharp · 桌面应用

基于 `fe/` 原型 (prototypes-v2 + base-resource) 实现的 Tauri + React + TS 前端应用。
设计语言:Claude 暖奶油编辑风(cream canvas · coral 稀缺点缀 · dark-navy code-window · serif 展示标题)。

## 运行

```bash
npm install
npm run dev        # 纯前端 (浏览器, http://localhost:1420)
npm run tauri dev  # 桌面应用 (需 Rust 工具链)
npm run build      # tsc + vite 产物检查
```

## 功能(全部前端内存态,localStorage 持久化,无后端)

| 模块 | 说明 |
|---|---|
| 知识库 | 卡片网格 · 筛选 · 收藏 · 详情面板(摘要/内容/反向链接) |
| 编辑器 | 暗色 raw-md 高亮 · `@` 两步插链(选卡→输关系)· wikilink 整块删除 · 实时预览(补全原型未做的分屏) |
| 知识审核 | 待审队列(新建/更新)· 行级 diff 双栏 · 编辑提议 · 通过即入库 · 拒绝 |
| 知识图谱 | force-directed · 数据由 wikilink 推导 · 断链节点虚线 · 悬停邻域高亮 · 力学参数面板 |
| 搜索 | ⌘K 面板 · 标题+摘要匹配 · 键盘全程可达 |
| Sheets | 新建卡片 / 设置 / 断链创建(macOS 顶部滑入) |

## 快捷键

`⌘K` 搜索 · `⌘N` 新建 · `⌘B` 侧栏 · `⌘1/2/3` 视图 · `Esc` 逐层关闭

深链:`#/kb` `#/review` `#/graph` `#/kb/<path>`(刷新保持现场)

## 数据流

卡片 body(raw markdown)是唯一数据源:
wikilink `[.knowledges/path|relation]` → 反向链接 / 图谱 / 断链 全部由此推导。
编辑与新建不直接入库,统一走 **待审队列 → 审核通过** 的生命周期。

## 设计令牌

`src/styles/tokens.css` + `components.css` 原样移植自 `fe/base-resource`(单一源);
`app.css` 只做 Tailwind v4 `@theme` 映射与页面专属样式。组件内禁止内联 hex。
