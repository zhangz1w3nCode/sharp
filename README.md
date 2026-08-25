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

## Why sharp

Coding agents accumulate knowledge scattered across Markdown files, ad-hoc notes, and directory conventions. sharp turns these files into a coherent, queryable, graph-connected system — with the filesystem as the single source of truth and the index as a rebuildable accelerator.

- **Filesystem authoritative** — All knowledge lives in `.md` files. The SQLite index is only an accelerator and can be fully rebuilt at any time with `index --build`; deleting it loses nothing.
- **Read-write separation** — Write commands (`add` / `update` / `rm`) land to the file first, then incrementally upsert the index; read commands (`search` / `links` / `traverse`) only query the index and never scan the filesystem.
- **Dual access paths** — Agents reach `sharp-core` through skill → CLI; humans reach it through the Tauri desktop app. Both converge on the same atomic operations.
- **Review-gated quality** — New documents start as `pending` (unindexed, unsearchable) and only enter the index after review validates them. `update` reverts a document back to pending.
- **Graph-native** — Documents connect via WikiLinks with labeled relationships. Forward/reverse links, BFS traversal, and the visual graph all derive from a single source — no separate graph store.

## Features

### Architecture

sharp is a Rust workspace with three crates, each with a single responsibility:

| Layer | Crate | Directory | Responsibility |
|-------|-------|-----------|----------------|
| core | `sharp-core` | `crates/core` | Atomic operations: SQLite index, frontmatter / WikiLink parsing, petgraph graph algorithms |
| cli | `sharp-cli` | `crates/cli` | Agent access layer: clap subcommands, pretty-printed JSON output |
| gui | `sharp` | `crates/gui` | Human access layer: Tauri + React desktop app |

### CLI Commands

```bash
sharp <command> [options]                    # knowledge base defaults to CWD/.knowledges
sharp --kb-root <path> <command>             # specify a knowledge base root
sharp --help                                 # full command list
```

| Command | Purpose |
|---------|---------|
| `init` | Initialize knowledge base + root document + INDEX.md |
| `create-domain` | Create a domain / sub-domain directory |
| `domains` | List domains |
| `rename-domain` | Rename a domain directory |
| `add` | Create a document (pending, awaiting review) |
| `update` | Update a document (reverts to pending) |
| `rm` | Delete a document (moved to `.trash-box`) |
| `trashbox` | Recycle bin: list / restore |
| `index` | Index management: `--build` / `--status` / `--tree` / `--flat` |
| `search` | Full-text search (FTS5 trigram tokenizer, bm25-weighted) |
| `show` | Show document frontmatter + body |
| `links` | Forward / reverse WikiLinks |
| `traverse` | Graph traversal (BFS, with edge relations) |
| `tags` | Tag listing |
| `doctor` | Health check |
| `stats` | Overview statistics |

### Review Mechanism

Document lifecycle: `add` / `init` → **pending** (not indexed, not searchable) → review approves → **validated** (indexed, searchable & traversable) → `update` reverts to pending.

Only `validated` documents are written to the index, and FTS5 search further filters on `status = 'validated'`. Review happens in the desktop app's Review view.

### Data Model

Each `.md` document opens with YAML frontmatter:

```yaml
---
name: document-name
summary: short abstract
domain: domain        # auto-derived from path
tags: [tag1, tag2]
status: validated     # pending | validated
---
```

Relationships between documents use WikiLinks: `[[.knowledges/path|relation]]`. Reverse links, the graph, and broken-link detection all derive from this single convention.

The SQLite index (`.sharp_index.sqlite`) has five tables: `docs` (only validated) / `tags` / `links` / `docs_fts` (FTS5, trigram tokenizer) / `meta`.

### Knowledge Graph

The desktop app renders the entire knowledge network as an interactive force-directed graph, with all edges derived from WikiLinks.

- Custom force-directed physics simulation (repulsion, spring attraction, centripetal force, velocity decay)
- Click any node to jump to its detail; broken (unresolved) links are flagged
- Hover a node to highlight its neighbors and dim the rest, with a preview popup
- Node radius scales with in-degree — highly referenced entries stand out visually
- Configurable physics: centripetal force, repulsion, attraction, link distance
- Display controls: arrow toggle, label fade, node size, edge thickness
- Pan and zoom (wheel + drag), draggable pinned nodes

### Desktop App

A Tauri + React desktop app with a titlebar-overlay window and keyboard-driven navigation (`⌘N` new, `⌘B` sidebar, `⌘1`–`⌘5` switch views, `Esc` hierarchical close).

Five views:

- **Knowledge Base** — card grid with a detail panel: inline editor (title / summary / body) and rendered Markdown preview, plus a backlinks list
- **Review** — review queue for pending documents (approve → validated)
- **Graph** — the interactive force-directed knowledge graph
- **Search** — full-text search results with surrounding context
- **Stats Overview** — KPI cards, 14-day creation trend, source distribution, 12-week activity heatmap, and a recent-activity feed

A global file-tree sidebar and modal sheets (broken-link inspector) round out the interface.

## Tech Stack

| Category | Technology | Version |
|----------|-----------|---------|
| Language | Rust | 2021 edition (MSRV 1.77.2) |
| CLI framework | clap | 4 |
| Storage | rusqlite (bundled, FTS5) | 0.32 |
| Graph | petgraph | 0.7 |
| Serialization | serde / serde_json / serde_yaml | 1 / 1 / 0.9 |
| Regex | regex | 1 |
| Filesystem walk | walkdir | 2 |
| Datetime | chrono | 0.4 |
| Errors | thiserror | 1 |
| Desktop | Tauri | 2 |
| Frontend | React | 18.3 |
| Language | TypeScript | 5.6 |
| Build tool | Vite | 6 |
| Styling | Tailwind CSS | 4 |
| Icons | lucide-react | 0.525 |
| Tauri FS plugin | @tauri-apps/plugin-fs | 2.5 |
| Package manager | npm | — |

## Getting Started

### Prerequisites

- **Rust** toolchain (rustup + cargo; MSRV 1.77.2)
- **Node.js** >= 18 + npm
- **Tauri** system dependencies (macOS: Xcode Command Line Tools)

### Installation

```bash
git clone https://github.com/zhangz1w3nCode/sharp.git
cd sharp

# Build the CLI
cargo build -p sharp-cli --release
ln -sf "$(pwd)/target/release/sharp" ~/.local/bin/sharp   # optional: put on PATH
```

### Development

```bash
# CLI (rebuild on change)
cargo build -p sharp-cli

# Desktop app (Vite dev server + Tauri hot-reload)
cd crates/gui
npm install
npm run tauri dev
```

The desktop dev server runs at `http://localhost:1420` with Tauri hot-reload.

### Build

```bash
# CLI release binary -> target/release/sharp
cargo build -p sharp-cli --release

# Desktop app bundle
cd crates/gui
npm run tauri build
```

Build outputs:

- **CLI**: `target/release/sharp`
- **Desktop**: `crates/gui/src-tauri/target/release/bundle/` (macOS `.app` + DMG; Windows NSIS installer; Linux `.deb` / `.AppImage`, depending on build platform)

## Project Structure

```
sharp/
├── crates/
│   ├── core/                # sharp-core — atomic operations (index, parser, graph)
│   ├── cli/                 # sharp-cli — agent access layer (bin: sharp)
│   └── gui/                 # sharp — desktop app (Tauri + React)
│       ├── src/             # frontend (views, components, lib, styles)
│       └── src-tauri/       # Rust backend + icons + tauri.conf.json
├── assets/                  # README logo
├── Cargo.toml               # workspace
└── LICENSE                  # MIT
```

## Data Storage

All knowledge is stored as plain Markdown files (with YAML frontmatter) inside the knowledge base directory, defaulting to `CWD/.knowledges`:

```
your-project/
└── .knowledges/                # knowledge base root (switchable via --kb-root)
    ├── INDEX.md                # generated index document
    ├── domain/                 # domain directories (create-domain)
    │   └── doc.md              # one document per .md file
    ├── .trash-box/             # recycle bin (rm -> here; trashbox restore)
    └── .sharp_index.sqlite     # SQLite index (rebuildable: index --build)
```

Each document is a standard Markdown file with optional YAML frontmatter, so you can version-control your knowledge base with Git and edit it with any text editor. The SQLite index is an accelerator — delete it and run `index --build` to reconstruct it from the files.

## License

[MIT](./LICENSE)
