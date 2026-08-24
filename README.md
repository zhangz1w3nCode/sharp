# sharp

> Agentic Knowledge Base — 面向 Agent 的确定性知识库系统。以 Markdown 文件为唯一事实源，通过 SQLite 索引、FTS5 全文搜索与 petgraph 图算法提供文件级与图级操作。名称 sharp 对应 # 符号。

## 核心理念

- **文件系统权威**：所有知识沉淀在 `.md` 文件中，索引只是加速器，可随时全量重建
- **读写分离**：写命令先落地文件再增量更新索引，读命令只读索引不扫文件系统
- **双接入路径**：Agent 经 skill → CLI → core；Human 经桌面应用 → core，统一汇聚到 sharp-core 原子能力

## 架构

| 层 | crate | 目录 | 职责 |
|---|---|---|---|
| core | `sharp-core` | `crates/core` | 核心原子操作：SQLite 索引、frontmatter/wikilink 解析、petgraph 图算法 |
| cli | `sharp-cli` | `crates/cli` | Agent 接入层：clap 子命令，输出 pretty-printed JSON |
| gui | `sharp` | `crates/gui` | Human 接入层：Tauri + React 桌面应用 |

## CLI 命令

```bash
sharp <command> [options]                    # 知识库默认: CWD/.knowledges
sharp --kb-root <path> <command>             # 指定知识库根目录
sharp --help                                 # 查看完整命令列表
```

| 命令 | 用途 |
|---|---|
| `init` | 初始化知识库 + 根文档 + INDEX.md |
| `create-domain` | 创建领域/子领域目录 |
| `domains` | 列出领域 |
| `rename-domain` | 重命名领域 |
| `add` | 创建文档（pending，待审核） |
| `update` | 更新文档（回退 pending） |
| `rm` | 删除文档（移入 .trash-box） |
| `trashbox` | 回收站：列出/恢复 |
| `index` | 索引管理：--build / --status / --tree / --flat |
| `search` | 全文搜索（FTS5 trigram，bm25 加权） |
| `show` | 显示文档 frontmatter + 正文 |
| `links` | 正向/反向链接 |
| `traverse` | 图遍历（BFS，带边关系） |
| `tags` | 标签列表 |
| `doctor` | 健康检查 |
| `stats` | 概览统计 |

## 审核机制

文档生命周期：`add`/`init` → **pending**（不入索引、不可搜）→ review 审核通过 → **validated**（入索引、可搜/遍历）→ `update` 回退 pending。

只有 `validated` 文档写入索引。审核经桌面应用完成。

## 数据模型

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

文档间关系用 wikilink：`[[.knowledges/path|relation]]`，反向链接、图谱、断链全部由此推导。

SQLite 索引（`.sharp_index.sqlite`）五表：`docs`（仅 validated）/ `tags` / `links` / `docs_fts`（FTS5）/ `meta`。

## 技术栈

**后端**：Rust · clap · rusqlite (FTS5) · petgraph · serde · regex · walkdir · thiserror

**前端**：React · TypeScript · Tauri · Vite · Tailwind v4 · lucide-react

## 快速开始

### 前置条件

- Rust 工具链（rustup + cargo）
- Node.js + npm（桌面应用）
- Tauri 依赖（macOS: Xcode CLI tools）

### CLI

```bash
cargo build -p sharp-cli --release
ln -sf "$(pwd)/target/release/sharp" ~/.local/bin/sharp

sharp init mykb --summary "根文档" --content "正文"
sharp create-domain mykb/notes
sharp add mykb/notes/article.md --link-from mykb/mykb.md --relation "参考" \
  --summary "摘要" --tags [rust] --content "正文"
sharp index --build
sharp search "关键词"
```

### 桌面应用

```bash
cd crates/gui
npm install
npm run tauri dev    # 开发模式
npm run tauri build  # 打包 Sharp.app
```

五视图：知识库卡片网格、编辑器（raw-md + wikilink 插链）、审核队列、知识图谱、搜索。

## 项目结构

```
sharp/
├── crates/
│   ├── core/      # sharp-core 核心原子操作
│   ├── cli/       # sharp-cli Agent 接入层 (bin: sharp)
│   └── gui/       # sharp 桌面应用 (Tauri + React)
│       ├── src/        # 前端
│       └── src-tauri/  # Rust 后端
├── .knowledges/   # 知识库数据 (Markdown + SQLite 索引)
└── Cargo.toml     # workspace
```

## License

[MIT](./LICENSE)
