//! akb - agentic-knowledge-base CLI(Rust 重写版,SQLite + petgraph 索引)。
//!
//! 索引一致性策略:所有操作走 CLI,写命令(add/update/add-batch/rm)实时
//! 单点增量更新索引(upsert_doc/remove_doc),读命令只读索引不扫文件系统。
//! 全量重建(index --build)作为手动兜底;未来可加定时任务自动 full_rebuild。

use std::path::Path;

use clap::{Parser, Subcommand};
use serde_json::{json, Value};


use akb_core::commands::{health, search, write};
use akb_core::db::IndexDb;
use akb_core::error::KbError;
use akb_core::util::{error_json, output_json};

#[derive(Parser)]
#[command(name = "kb", about = "knowledge-base CLI: 确定性文件级/图级操作")]
struct Cli {
    /// 知识库根目录(默认: CWD/.knowledges)
    #[arg(long, global = true)]
    kb_root: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 索引管理:重建/状态/树形/平铺
    Index(IndexArgs),
    /// 正向/反向 wiki-link
    Links {
        /// 源文档路径
        #[arg(long = "from")]
        from: String,
        /// 查反向链接(inlinks)
        #[arg(long)]
        reverse: bool,
    },
    /// 图遍历(BFS,带边标签,按 target 去重)
    Traverse {
        /// 起始文档路径
        #[arg(long = "from")]
        from: String,
        /// 最大跳数(默认 2)
        #[arg(short = 'j', long, default_value = "2")]
        jumps: usize,
        /// 双向遍历(out+in)
        #[arg(long)]
        bidir: bool,
        /// 只保留边标签等于此值的路径(精确匹配)
        #[arg(long = "label-filter")]
        label_filter: Option<String>,
    },
    /// 列出全部 tag 或查指定 tag 的文档
    Tags {
        /// tag 名称(省略则列全部)
        tag: Option<String>,
    },
    /// 全文搜索(FTS5 覆盖 name + summary + body,带列权重 + 上下文)
    Search {
        /// 搜索关键词
        keyword: String,
        /// 只返回命中数最多的前 N 个文件
        #[arg(long)]
        top: Option<usize>,
        /// 匹配行上下文行数(上下各 N 行,默认 5)
        #[arg(long, default_value_t = 5)]
        context: usize,
    },
    /// 显示文档 frontmatter + 正文
    Show {
        /// 文档路径
        doc: String,
        /// 只返回 frontmatter summary(快速判断相关性)
        #[arg(long)]
        summary: bool,
    },
    /// 创建新文档 + 在父文档建立带标签 wiki-link
    Add(AddArgs),
    /// 删除影响报告(只报告,不删文件)
    Rm {
        /// 文档路径
        doc: String,
    },
    /// 更新文档(content/append/summary/name/tags/add-link)
    Update(UpdateArgs),
    /// 从 JSON 数组批量创建文档
    AddBatch {
        /// JSON 数组文件路径
        #[arg(long = "from-file")]
        from_file: String,
    },
    /// 初始化知识库目录 + 根文档 + INDEX.md
    Init(InitArgs),
    /// 健康检查(详细)
    Doctor,
    /// 知识库精简概览(文档数/tag/domains/health_score/last_modified)
    Stats,
}

#[derive(clap::Args)]
#[group(required = true, multiple = false)]
struct IndexArgs {
    /// 全量重建索引
    #[arg(long)]
    build: bool,
    /// 查看并修复索引状态
    #[arg(long)]
    status: bool,
    /// 返回树形嵌套结构(目录套目录)
    #[arg(long)]
    tree: bool,
    /// 返回平铺路径列表(每个文件一行完整路径)
    #[arg(long)]
    flat: bool,
}

#[derive(clap::Args)]
struct AddArgs {
    /// 新文档相对路径(如 zoloz/zoloz-xxx.md)
    doc_path: String,
    /// 父文档路径
    #[arg(long = "link-from")]
    link_from: String,
    /// 边的语义关系(如:业务背景)
    #[arg(long)]
    label: Option<String>,
    /// frontmatter name
    #[arg(long)]
    name: Option<String>,
    /// frontmatter summary(文档摘要,必需)
    #[arg(long)]
    summary: String,
    /// frontmatter description
    #[arg(long)]
    description: Option<String>,
    /// frontmatter category
    #[arg(long)]
    category: Option<String>,
    /// frontmatter tag(可多次)
    #[arg(long = "tags", value_name = "TAG")]
    tags: Vec<String>,
    /// 文档正文内容
    #[arg(long, conflicts_with = "content_file")]
    content: Option<String>,
    /// 从文件读取文档正文
    #[arg(long = "content-file", conflicts_with = "content")]
    content_file: Option<String>,
}

#[derive(clap::Args)]
struct InitArgs {
    /// 业务领域名(如 zoloz)
    domain: String,
    /// frontmatter name(默认用 domain)
    #[arg(long)]
    name: Option<String>,
    /// frontmatter summary(根文档摘要,必需)
    #[arg(long)]
    summary: String,
    /// frontmatter description(默认 "{domain} 业务领域根节点")
    #[arg(long)]
    description: Option<String>,
    /// frontmatter category(默认用 domain)
    #[arg(long)]
    category: Option<String>,
    /// frontmatter tag(可多次,默认 [domain])
    #[arg(long = "tags", value_name = "TAG")]
    tags: Vec<String>,
    /// 根文档正文内容
    #[arg(long, conflicts_with = "content_file")]
    content: Option<String>,
    /// 从文件读取根文档正文
    #[arg(long = "content-file", conflicts_with = "content")]
    content_file: Option<String>,
}

#[derive(clap::Args)]
struct UpdateArgs {
    /// 文档路径
    doc: String,
    /// 替换正文
    #[arg(long, conflicts_with = "append")]
    content: Option<String>,
    /// 追加到正文末尾
    #[arg(long, conflicts_with = "content")]
    append: Option<String>,
    /// 更新 frontmatter summary
    #[arg(long)]
    summary: Option<String>,
    /// 更新 frontmatter name
    #[arg(long)]
    name: Option<String>,
    /// 更新 frontmatter tag(可多次)
    #[arg(long = "tags", value_name = "TAG")]
    tags: Vec<String>,
    /// 追加 wiki-link(需配合 --to)
    #[arg(long = "add-link")]
    add_link: bool,
    /// --add-link 的目标文档路径
    #[arg(long)]
    to: Option<String>,
    /// --add-link 的边标签
    #[arg(long)]
    label: Option<String>,
}

/// 解析 --content 或 --content-file,返回正文内容。
fn resolve_content(content: &Option<String>, content_file: &Option<String>) -> Result<String, String> {
    if let Some(c) = content {
        return Ok(c.clone());
    }
    if let Some(f) = content_file {
        return std::fs::read_to_string(f).map_err(|e| format!("reading content file: {}", e));
    }
    Err("content is required (use --content or --content-file)".to_string())
}

fn open_db(kb_root_abs: &str) -> IndexDb {
    match IndexDb::open(kb_root_abs) {
        Ok(db) => db,
        Err(e) => {
            output_json(
                &json!({
                    "error": format!("failed to open index: {}", e),
                    "hint": "try running 'akb index --build' to rebuild the index"
                }),
            );
            std::process::exit(2);
        }
    }
}

/// index 子命令分发(build/status/tree/flat)。
fn run_index(db: &mut IndexDb, kb_root_abs: &str, args: IndexArgs) -> Result<Value, KbError> {
    if args.build {
        let stats = db.full_rebuild(kb_root_abs)
            .map_err(|e| KbError::Other(format!("index build failed: {}", e)))?;
        Ok(json!({"command": "index", "mode": "build", "indexed": stats.indexed}))
    } else if args.status {
        let repair = db.repair_stale(kb_root_abs)
            .map_err(|e| KbError::Other(format!("repair stale failed: {}", e)))?;
        let status = db.status(kb_root_abs)
            .map_err(|e| KbError::Other(format!("status failed: {}", e)))?;
        Ok(json!({
            "command": "index",
            "mode": "status",
            "repair": {
                "added": repair.added,
                "updated": repair.updated,
                "removed": repair.removed,
            },
            "status": {
                "index_exists": status.index_exists,
                "index_file": status.index_file,
                "indexed_count": status.indexed_count,
                "filesystem_count": status.filesystem_count,
                "stale_count": status.stale_count,
                "missing_count": status.missing_count,
                "new_count": status.new_count,
            },
        }))
    } else if args.tree {
        search::cmd_index(db, kb_root_abs, false)
    } else {
        search::cmd_index(db, kb_root_abs, true)
    }
}

fn main() {
    let cli = Cli::parse();
    let kb_root_abs = match &cli.kb_root {
        Some(r) => std::fs::canonicalize(r)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| {
                // canonicalize 失败(目录可能不存在),用绝对路径
                let abs = std::env::current_dir()
                    .map(|cd| cd.join(r))
                    .unwrap_or_else(|_| Path::new(r).to_path_buf());
                abs.to_string_lossy().to_string()
            }),
        None => {
            // 默认 CWD/.knowledges
            let cd = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
            cd.join(".knowledges").to_string_lossy().to_string()
        }
    };

    let kb_exists = Path::new(&kb_root_abs).is_dir();
    // 只有 init 命令允许目录不存在
    if !kb_exists {
        let is_init = matches!(cli.command, Commands::Init(_));
        if !is_init {
            output_json(&json!({
                "error": format!("knowledge base directory not found: {}", kb_root_abs),
                "hint": "use --kb-root <path> to specify, or run \"kb init <domain>\" first"
            }));
            std::process::exit(2);
        }
    }

    let result: Result<Value, KbError> = match cli.command {
        Commands::Init(args) => match resolve_content(&args.content, &args.content_file) {
            Ok(content) => write::cmd_init(
                &kb_root_abs, &args.domain, args.name.as_deref(), &args.summary,
                args.description.as_deref(), args.category.as_deref(), args.tags, &content,
            ),
            Err(e) => Err(KbError::Other(e)),
        },
        command => {
            let mut db = open_db(&kb_root_abs);
            match command {
                Commands::Index(args) => run_index(&mut db, &kb_root_abs, args),
                Commands::Links { from, reverse } => {
                    search::cmd_links(&mut db, &kb_root_abs, &from, reverse)
                }
                Commands::Traverse { from, jumps, bidir, label_filter } => {
                    search::cmd_traverse(&mut db, &kb_root_abs, &from, jumps, bidir, label_filter.as_deref())
                }
                Commands::Tags { tag } => search::cmd_tags(&mut db, &kb_root_abs, tag.as_deref()),
                Commands::Search { keyword, top, context } => {
                    search::cmd_search(&mut db, &kb_root_abs, &keyword, top, context)
                }
                Commands::Show { doc, summary } => search::cmd_show(&kb_root_abs, &doc, summary),
                Commands::Add(args) => match resolve_content(&args.content, &args.content_file) {
                    Ok(content) => write::cmd_add(
                        &mut db, &kb_root_abs, &args.doc_path, &args.link_from, args.label.as_deref(),
                        args.name.as_deref(), &args.summary, args.description.as_deref(),
                        args.category.as_deref(), args.tags, &content,
                    ),
                    Err(e) => Err(KbError::Other(e)),
                },
                Commands::Rm { doc } => write::cmd_rm(&mut db, &kb_root_abs, &doc),
                Commands::Update(args) => write::cmd_update(
                    &mut db, &kb_root_abs, &args.doc, args.content.as_deref(), args.append.as_deref(),
                    args.summary.as_deref(), args.name.as_deref(), args.tags, args.add_link,
                    args.to.as_deref(), args.label.as_deref(),
                ),
                Commands::AddBatch { from_file } => write::cmd_add_batch(&mut db, &kb_root_abs, &from_file),
                Commands::Doctor => health::cmd_doctor(&mut db, &kb_root_abs),
                Commands::Stats => health::cmd_stats(&mut db, &kb_root_abs),
                Commands::Init(_) => unreachable!(),
            }
        }
    };
    let code = match result {
        Ok(v) => { output_json(&v); 0 }
        Err(e) => { error_json(&e.to_string()); 1 }
    };
    std::process::exit(code);
}
