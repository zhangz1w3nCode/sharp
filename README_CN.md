<div align="center">

<img src="./assets/logo.svg" alt="sharp logo" width="144" />

---

**Sharp**


<p align="center">
  <a href="https://opensource.org/licenses/MIT"><img alt="License: MIT" src="https://img.shields.io/badge/License-MIT-yellow.svg"></a>
  <a href="https://github.com/zhangz1w3nCode/sharp/stargazers"><img alt="GitHub stars" src="https://img.shields.io/github/stars/zhangz1w3nCode/sharp?style=social"></a>
  <a href="https://github.com/zhangz1w3nCode/sharp"><img alt="Last commit" src="https://img.shields.io/github/last-commit/zhangz1w3nCode/sharp"></a>
</p>

[English](./README.md) | [中文](./README_CN.md)

</div>

---

## 为什么是 sharp

编码 Agent 积累的知识散落在 Markdown 文件、临时笔记与目录约定中。sharp 把这些文件整合为一个连贯、可查询、图谱互联的系统——以文件系统为唯一事实源，索引只是可重建的加速器。

- **文件系统权威**：所有知识沉淀在 `.md` 文件中。SQLite 索引只是加速器，随时可用 `index --build` 全量重建，删除索引不丢失任何内容。
- **读写分离**：写命令（`add` / `update` / `rm`）先落地文件，再增量 upsert 索引；读命令（`search` / `links` / `traverse`）只读索引，不扫文件系统。
- **双接入路径**：Agent 经 skill → CLI 接入 `sharp-core`；人类经 Tauri 桌面应用接入。两者汇聚到同一套原子操作。
- **审核门控**：新文档以 `pending`（不入索引、不可搜）起步，审核通过后才入索引为 `validated`。`update` 会把文档回退为 pending。
- **图谱原生**：文档间用带关系标签的 WikiLink 连接。正向/反向链接、BFS 遍历、可视化图谱全部由此单一来源推导，无需独立图存储。

## 功能

### 架构

sharp 是一个 Rust workspace，三个 crate 各司其职：

| 层 | crate | 目录 | 职责 |
|---|---|---|---|
| core | `sharp-core` | `crates/core` | 核心原子操作：SQLite 索引、frontmatter / WikiLink 解析、petgraph 图算法 |
| cli | `sharp-cli` | `crates/cli` | Agent 接入层：clap 子命令，输出 pretty-printed JSON |
| gui | `sharp` | `crates/gui` | Human 接入层：Tauri + React 桌面应用 |

### CLI 命令

```bash
sharp <command> [options]                    # 知识库默认: CWD/.knowledges
sharp --kb-root <path> <command>             # 指定知识库根目录
sharp --help                                 # 查看完整命令列表
```

| 命令 | 用途 |
|---|---|
| `init` | 初始化知识库 + 根文档 + INDEX.md |
| `create-domain` | 创建领域 / 子领域目录 |
| `domains` | 列出领域 |
| `rename-domain` | 重命名领域目录 |
| `add` | 创建文档（pending，待审核） |
| `update` | 更新文档（回退为 pending） |
| `rm` | 删除文档（移入 `.trash-box`） |
| `trashbox` | 回收站：列出 / 恢复 |
| `index` | 索引管理：`--build` / `--status` / `--tree` / `--flat` |
| `search` | 全文搜索（FTS5 trigram 分词，bm25 加权） |
| `show` | 显示文档 frontmatter + 正文 |
| `links` | 正向 / 反向 WikiLink |
| `traverse` | 图遍历（BFS，带边关系） |
| `tags` | 标签列表 |
| `doctor` | 健康检查 |
| `stats` | 概览统计 |

### 审核机制

文档生命周期：`add` / `init` → **pending**（不入索引、不可搜）→ 审核通过 → **validated**（入索引、可搜/可遍历）→ `update` 回退 pending。

只有 `validated` 文档写入索引，FTS5 搜索进一步过滤 `status = 'validated'`。审核在桌面应用的 Review 视图完成。

### 数据模型

每个 `.md` 文档以 YAML frontmatter 开头：

```yaml
---
name: 文档名称
summary: 摘要
domain: 领域        # 从路径自动推导
tags: [tag1, tag2]
status: validated  # pending | validated
---
```

文档间关系用 WikiLink：`[[.knowledges/path|relation]]`。反向链接、图谱、断链检测全部由此单一约定推导。

SQLite 索引（`.sharp_index.sqlite`）五张表：`docs`（仅 validated）/ `tags` / `links` / `docs_fts`（FTS5，trigram 分词）/ `meta`。

### 知识图谱

桌面应用把整个知识网络渲染为可交互的力导向图谱，所有边由 WikiLink 推导。

- 自研力导向物理模拟（斥力、弹簧引力、向心力、速度衰减）
- 点击任意节点跳转到详情；断链（未解析）会被标记
- 悬停节点高亮邻居并淡化其余节点，弹出预览卡
- 节点半径随入度缩放——被引用越多越醒目
- 物理参数可调：向心力、斥力、引力、连线距离
- 显示控制：箭头开关、标签淡出、节点大小、连线粗细
- 平移与缩放（滚轮 + 拖拽），节点可拖拽固定

### 桌面应用

Tauri + React 桌面应用，titlebar-overlay 窗口，键盘驱动导航（`⌘N` 新建、`⌘B` 侧栏、`⌘1`–`⌘5` 切换视图、`Esc` 逐层关闭）。

五个视图：

- **知识库**——卡片网格 + 详情面板：内联编辑器（标题 / 摘要 / 正文）与 Markdown 渲染预览，附反向链接列表
- **审核**——pending 文档审核队列（通过 → validated）
- **图谱**——可交互的力导向知识图谱
- **搜索**——全文搜索结果（带上下文）
- **统计概览**——KPI 卡片、近 14 天创建趋势、来源分布、近 12 周活跃度热力图、最近动态流

全局文件树侧栏与模态抽屉（断链检查器）补全整体界面。

## 技术栈

| 类别 | 技术 | 版本 |
|---|---|---|
| 语言 | Rust | 2021 edition（MSRV 1.77.2） |
| CLI 框架 | clap | 4 |
| 存储 | rusqlite（bundled, FTS5） | 0.32 |
| 图算法 | petgraph | 0.7 |
| 序列化 | serde / serde_json / serde_yaml | 1 / 1 / 0.9 |
| 正则 | regex | 1 |
| 文件遍历 | walkdir | 2 |
| 时间 | chrono | 0.4 |
| 错误 | thiserror | 1 |
| 桌面 | Tauri | 2 |
| 前端 | React | 18.3 |
| 语言 | TypeScript | 5.6 |
| 构建工具 | Vite | 6 |
| 样式 | Tailwind CSS | 4 |
| 图标 | lucide-react | 0.525 |
| Tauri FS 插件 | @tauri-apps/plugin-fs | 2.5 |
| 包管理器 | npm | — |

## 快速开始

### 前置条件

- **Rust** 工具链（rustup + cargo；MSRV 1.77.2）
- **Node.js** >= 18 + npm
- **Tauri** 系统依赖（macOS：Xcode Command Line Tools）

### 安装

```bash
git clone https://github.com/zhangz1w3nCode/sharp.git
cd sharp

# 构建 CLI
cargo build -p sharp-cli --release
ln -sf "$(pwd)/target/release/sharp" ~/.local/bin/sharp   # 可选：放到 PATH
```

### 开发

```bash
# CLI（变更后重新构建）
cargo build -p sharp-cli

# 桌面应用（Vite dev server + Tauri 热重载）
cd crates/gui
npm install
npm run tauri dev
```

桌面 dev server 运行在 `http://localhost:1420`，带 Tauri 热重载。

### 构建打包

```bash
# CLI release 二进制 → target/release/sharp
cargo build -p sharp-cli --release

# 桌面应用打包
cd crates/gui
npm run tauri build
```

构建产物：

- **CLI**：`target/release/sharp`
- **桌面应用**：`crates/gui/src-tauri/target/release/bundle/`（macOS `.app` + DMG；Windows NSIS 安装包；Linux `.deb` / `.AppImage`，取决于构建平台）

## 项目结构

```
sharp/
├── crates/
│   ├── core/                # sharp-core — 原子操作（索引、解析、图）
│   ├── cli/                 # sharp-cli — Agent 接入层（bin: sharp）
│   └── gui/                 # sharp — 桌面应用（Tauri + React）
│       ├── src/             # 前端（views / components / lib / styles）
│       └── src-tauri/       # Rust 后端 + icons + tauri.conf.json
├── assets/                  # README logo
├── Cargo.toml               # workspace
└── LICENSE                  # MIT
```

## 数据存储

所有知识以纯 Markdown 文件（带 YAML frontmatter）存储在知识库目录，默认 `CWD/.knowledges`：

```
your-project/
└── .knowledges/                # 知识库根目录（可用 --kb-root 切换）
    ├── INDEX.md                # 生成的索引文档
    ├── domain/                 # 领域目录（create-domain）
    │   └── doc.md              # 每个 .md 文件即一篇文档
    ├── .trash-box/             # 回收站（rm → 此处；trashbox restore 恢复）
    └── .sharp_index.sqlite     # SQLite 索引（可重建：index --build）
```

每个文档是带可选 YAML frontmatter 的标准 Markdown 文件，因此可以用 Git 版本控制你的知识库，用任意文本编辑器编辑。SQLite 索引只是加速器——删除后运行 `index --build` 即可从文件重建。

## License

[MIT](./LICENSE)
