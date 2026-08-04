//! db.rs - SQLite 索引管理(单一索引文件,FTS5 全文搜索)。

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::KbError;
use crate::index::scan_files;
use crate::parser::{parse_frontmatter, parse_wikilinks};

/// 索引数据库连接。
pub struct IndexDb {
    conn: Connection,
    index_path: PathBuf,
}

/// 全量重建统计。
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RebuildStats {
    pub indexed: usize,
    pub removed: usize,
}

/// 增量修复统计。
#[derive(Debug, Clone)]
pub struct RepairStats {
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
}

/// FTS5 搜索结果项。
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub path: String,
    pub name: String,
    pub summary: String,
    pub rank: f64,
}

/// 文档元数据(供 doctor/stats 使用)。
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DocMeta {
    pub path: String,
    pub name: String,
    pub summary: String,
    pub domain: String,
    pub has_frontmatter: bool,
    pub status: String,
    pub tags: Vec<String>,
    pub outlinks: Vec<(String, Option<String>)>,
    pub mtime_secs: i64,
    pub mtime_nanos: u32,
    pub size_bytes: u64,
}

/// 文档记录(读命令 show 使用,来自 docs 表)。
#[derive(Debug, Clone)]
pub struct DocRecord {
    pub path: String,
    pub name: String,
    pub summary: String,
    pub domain: String,
    pub has_frontmatter: bool,
    pub body: String,
}

/// 索引状态。
#[derive(Debug, Clone)]
pub struct IndexStatus {
    pub index_exists: bool,
    pub index_file: String,
    pub indexed_count: usize,
    pub filesystem_count: usize,
    pub stale_count: usize,
    pub missing_count: usize,
    pub new_count: usize,
}

impl IndexDb {
    /// 打开或创建索引文件。
    /// 打开或创建索引文件。版本迁移时自动重建索引避免空库。
    pub fn open(kb_root_abs: &str) -> Result<Self, KbError> {
        let index_path = Path::new(kb_root_abs).join(".akb_index.sqlite");
        let conn = Connection::open(&index_path).map_err(KbError::Sqlite)?;
        let mut db = IndexDb { conn, index_path };
        let migrated = db.ensure_schema().map_err(KbError::Sqlite)?;
        // 版本迁移后索引为空,自动重建避免读命令看到空库
        if migrated {
            db.full_rebuild(kb_root_abs).map_err(|e| KbError::Other(format!("index rebuild after migration: {}", e)))?;
        }
        Ok(db)
    }

    fn ensure_schema(&self) -> Result<bool, rusqlite::Error> {
        // 先建 meta 表,读取 schema_version
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )?;
        let current_version: String = self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'schema_version'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .unwrap_or_else(|| "0".to_string());

        let mut migrated = false;
        // schema v6: docs 表新增 domain 列,版本不匹配时重建
        if current_version != "6" {
            migrated = true;
            self.conn.execute_batch(
                "DROP TRIGGER IF EXISTS docs_ai;
                 DROP TRIGGER IF EXISTS docs_ad;
                 DROP TRIGGER IF EXISTS docs_au;
                 DROP TABLE IF EXISTS docs_fts;
                 DROP TABLE IF EXISTS tags;
                 DROP TABLE IF EXISTS links;
                 DROP TABLE IF EXISTS docs;",
            )?;
        }

        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS docs (
                path TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                summary TEXT NOT NULL,
                heading TEXT NOT NULL,
                body TEXT NOT NULL,
                tags_text TEXT NOT NULL,
                domain TEXT NOT NULL,
                has_frontmatter INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                mtime_secs INTEGER NOT NULL,
                mtime_nanos INTEGER NOT NULL,
                size_bytes INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS tags (
                doc_path TEXT NOT NULL REFERENCES docs(path) ON DELETE CASCADE,
                tag TEXT NOT NULL,
                PRIMARY KEY (doc_path, tag)
            );
            CREATE INDEX IF NOT EXISTS idx_tags_tag ON tags(tag);

            CREATE TABLE IF NOT EXISTS links (
                source TEXT NOT NULL,
                target TEXT NOT NULL,
                relation TEXT,
                PRIMARY KEY (source, target, relation)
            ) WITHOUT ROWID;
            CREATE INDEX IF NOT EXISTS idx_links_target ON links(target);

            CREATE VIRTUAL TABLE IF NOT EXISTS docs_fts USING fts5(
                path UNINDEXED,
                name,
                summary,
                heading,
                body,
                tags_text,
                content='docs',
                content_rowid='rowid',
                tokenize='trigram'
            );

            CREATE TRIGGER IF NOT EXISTS docs_ai AFTER INSERT ON docs BEGIN
              INSERT INTO docs_fts(rowid, path, name, summary, heading, body, tags_text)
              VALUES (new.rowid, new.path, new.name, new.summary, new.heading, new.body, new.tags_text);
            END;

            CREATE TRIGGER IF NOT EXISTS docs_ad AFTER DELETE ON docs BEGIN
              INSERT INTO docs_fts(docs_fts, rowid, path, name, summary, heading, body, tags_text)
              VALUES ('delete', old.rowid, old.path, old.name, old.summary, old.heading, old.body, old.tags_text);
            END;

            CREATE TRIGGER IF NOT EXISTS docs_au AFTER UPDATE ON docs BEGIN
              INSERT INTO docs_fts(docs_fts, rowid, path, name, summary, heading, body, tags_text)
              VALUES ('delete', old.rowid, old.path, old.name, old.summary, old.heading, old.body, old.tags_text);
              INSERT INTO docs_fts(rowid, path, name, summary, heading, body, tags_text)
              VALUES (new.rowid, new.path, new.name, new.summary, new.heading, new.body, new.tags_text);
            END;

            PRAGMA foreign_keys = ON;

            INSERT OR REPLACE INTO meta(key, value) VALUES ('schema_version', '6');

            ANALYZE;
            "
        )?;
        Ok(migrated)
    }

    /// 全量重建索引。
    pub fn full_rebuild(&mut self, kb_root_abs: &str) -> Result<RebuildStats, KbError> {
        let files = scan_files(kb_root_abs);
        let tx = self.conn.transaction()?;

        // 清空索引
        tx.execute("DELETE FROM docs", [])?;
        tx.execute("DELETE FROM links", [])?;

        for rel in &files {
            Self::upsert_doc_in_tx(&tx, kb_root_abs, rel)?;
        }

        // indexed = validated 文档数(非文件总数);pending 文档不写入索引
        let indexed: i64 = tx.query_row("SELECT COUNT(*) FROM docs", [], |row| row.get(0))?;
        // 清理指向未索引文档(pending)的 links
        tx.execute("DELETE FROM links WHERE target NOT IN (SELECT path FROM docs)", [])?;

        tx.commit()?;
        Ok(RebuildStats {
            indexed: indexed as usize,
            removed: 0,
        })
    }

    /// 基于 mtime/size 的增量修复。
    pub fn repair_stale(&mut self, kb_root_abs: &str) -> Result<RepairStats, KbError> {
        let files = scan_files(kb_root_abs);
        let on_disk: HashMap<String, (i64, u32, u64)> = files
            .iter()
            .filter_map(|rel| file_stat(kb_root_abs, rel).map(|s| (rel.clone(), s)))
            .collect();

        let mut indexed: HashMap<String, (i64, u32, u64)> = HashMap::new();
        {
            let mut stmt = self
                .conn
                .prepare("SELECT path, mtime_secs, mtime_nanos, size_bytes FROM docs")?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, u32>(2)?,
                    row.get::<_, i64>(3)? as u64,
                ))
            })?;
            for row in rows {
                let (path, secs, nanos, size) = row?;
                indexed.insert(path, (secs, nanos, size));
            }
        }

        let tx = self.conn.transaction()?;
        let mut added = 0;
        let mut updated = 0;
        let mut removed = 0;

        // 删除已不在文件系统的文档及其正向链接
        for path in indexed.keys() {
            if !on_disk.contains_key(path) {
                tx.execute("DELETE FROM docs WHERE path = ?1", params![path])?;
                tx.execute("DELETE FROM links WHERE source = ?1", params![path])?;
                removed += 1;
            }
        }

        // 新增或更新
        for (path, disk) in &on_disk {
            let should_upsert = match indexed.get(path) {
                None => {
                    added += 1;
                    true
                }
                Some((secs, nanos, size)) => {
                    if disk.0 != *secs || disk.1 != *nanos || disk.2 != *size {
                        updated += 1;
                        true
                    } else {
                        false
                    }
                }
            };
            if should_upsert {
                Self::upsert_doc_in_tx(&tx, kb_root_abs, path)?;
            }
        }

        tx.execute("DELETE FROM links WHERE target NOT IN (SELECT path FROM docs)", [])?;

        tx.commit()?;
        Ok(RepairStats {
            added,
            updated,
            removed,
        })
    }

    /// 单文档 upsert(公开 API)。
    pub fn upsert_doc(&mut self, kb_root_abs: &str, rel_path: &str) -> Result<(), KbError> {
        let tx = self.conn.transaction()?;
        Self::upsert_doc_in_tx(&tx, kb_root_abs, rel_path)?;
        tx.execute("DELETE FROM links WHERE source = ?1 AND target NOT IN (SELECT path FROM docs)", params![rel_path])?;
        tx.commit()?;
        Ok(())
    }

    fn upsert_doc_in_tx(
        tx: &rusqlite::Transaction,
        kb_root_abs: &str,
        rel_path: &str,
    ) -> Result<(), KbError> {
        let abs = Path::new(kb_root_abs).join(rel_path.replace('/', std::path::MAIN_SEPARATOR_STR));
        let text = std::fs::read_to_string(&abs)?;
        let meta = file_stat(kb_root_abs, rel_path)
            .ok_or_else(|| KbError::Other(format!("stat failed: {}", rel_path)))?;
        let (fm, body, has_fm) = parse_frontmatter(&text);
        // domain 从文档路径推导(single source of truth),不依赖 frontmatter
        let domain = rel_path.rsplit_once('/').map(|(d, _)| d.to_string()).unwrap_or_default();
        let links = parse_wikilinks(&text, ".knowledges");
        let tags: Vec<String> = fm
            .tags
            .into_iter()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();
        // heading: body 中以 # 开头的行(标题),单独提取供 FTS5 加权
        let heading: String = body
            .split('\n')
            .filter(|line| line.trim_start().starts_with('#'))
            .collect::<Vec<&str>>()
            .join("\n");
        let tags_text = tags.join(" ");

        if fm.status == "validated" {
            tx.execute(
                "INSERT INTO docs(path, name, summary, heading, body, tags_text, domain, has_frontmatter, status, mtime_secs, mtime_nanos, size_bytes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                 ON CONFLICT(path) DO UPDATE SET
                   name = excluded.name,
                   summary = excluded.summary,
                   heading = excluded.heading,
                   body = excluded.body,
                   tags_text = excluded.tags_text,
                   domain = excluded.domain,
                   has_frontmatter = excluded.has_frontmatter,
                   status = excluded.status,
                   mtime_secs = excluded.mtime_secs,
                   mtime_nanos = excluded.mtime_nanos,
                   size_bytes = excluded.size_bytes",
                params![
                    rel_path,
                    fm.name,
                    fm.summary,
                    heading,
                    body,
                    tags_text,
                    domain,
                    if has_fm { 1 } else { 0 },
                    fm.status,
                    meta.0,
                    meta.1,
                    meta.2 as i64,
                ],
            )?;

            // 刷新 tags
            tx.execute("DELETE FROM tags WHERE doc_path = ?1", params![rel_path])?;
            for tag in &tags {
                tx.execute(
                    "INSERT OR IGNORE INTO tags(doc_path, tag) VALUES (?1, ?2)",
                    params![rel_path, tag],
                )?;
            }

            // 刷新 links(source=rel_path)
            tx.execute("DELETE FROM links WHERE source = ?1", params![rel_path])?;
            for (target, relation) in &links {
                tx.execute(
                    "INSERT OR IGNORE INTO links(source, target, relation) VALUES (?1, ?2, ?3)",
                    params![rel_path, target, relation.as_deref().unwrap_or("")],
                )?;
            }
        } else {
            // status != validated: 从索引中移除(如果存在),FTS5 触发器自动同步
            tx.execute("DELETE FROM docs WHERE path = ?1", params![rel_path])?;
            tx.execute("DELETE FROM tags WHERE doc_path = ?1", params![rel_path])?;
            tx.execute("DELETE FROM links WHERE source = ?1", params![rel_path])?;
            tx.execute("DELETE FROM links WHERE target = ?1", params![rel_path])?;
        }
        Ok(())
    }

    /// 从索引中移除文档。
    pub fn remove_doc(&mut self, rel_path: &str) -> Result<(), KbError> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM docs WHERE path = ?1", params![rel_path])?;
        tx.execute("DELETE FROM tags WHERE doc_path = ?1", params![rel_path])?;
        tx.execute("DELETE FROM links WHERE source = ?1", params![rel_path])?;
        tx.execute("DELETE FROM links WHERE target = ?1", params![rel_path])?;
        tx.commit()?;
        Ok(())
    }

    /// 批量从索引读取多篇文档正文(1 次 SQL 替代 N 次,用于 search context)。
    pub fn get_doc_bodies(&self, paths: &[&str]) -> HashMap<String, String> {
        let mut map = HashMap::with_capacity(paths.len());
        if paths.is_empty() {
            return map;
        }
        let placeholders = (0..paths.len())
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!("SELECT path, body FROM docs WHERE path IN ({})", placeholders);
        let mut stmt = match self.conn.prepare(&sql) {
            Ok(s) => s,
            Err(_) => return map,
        };
        let rows = stmt.query_map(rusqlite::params_from_iter(paths.iter()), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        });
        if let Ok(rows) = rows {
            for (p, b) in rows.filter_map(Result::ok) {
                map.insert(p, b);
            }
        }
        map
    }

    /// 按 path 读取单篇文档(docs 表,只含 validated)。
    /// 返回 None 表示文档不在索引中(不存在或 pending)。
    pub fn get_doc(&self, path: &str) -> Result<Option<DocRecord>, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT path, name, summary, domain, has_frontmatter, body FROM docs WHERE path = ?1",
                params![path],
                |row| {
                    Ok(DocRecord {
                        path: row.get(0)?,
                        name: row.get(1)?,
                        summary: row.get(2)?,
                        domain: row.get(3)?,
                        has_frontmatter: row.get::<_, i64>(4)? != 0,
                        body: row.get(5)?,
                    })
                },
            )
            .optional()
    }

    /// 按 path 读取文档全部 tags(tags 表,每行一个,排序稳定)。
    pub fn tags_for_doc(&self, path: &str) -> Result<Vec<String>, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT tag FROM tags WHERE doc_path = ?1 ORDER BY tag")?;
        let rows = stmt.query_map(params![path], |row| row.get(0))?;
        rows.collect()
    }

    /// 读取索引中全部文档 path(只含 validated,排序稳定)。
    pub fn all_doc_paths(&self) -> Result<Vec<String>, rusqlite::Error> {
        let mut stmt = self.conn.prepare("SELECT path FROM docs ORDER BY path")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        rows.collect()
    }

    /// FTS5 搜索 name + summary + heading + body + tags。
    ///
    /// 用户查询用双引号包裹成 FTS5 phrase(避免 `-`/`*` 等被当作语法);
    /// trigram tokenizer 支持子串匹配,要求查询 >= 3 字符。
    /// 中文 2 字词无法命中(trigram 最小 3 字符),后续如需支持可加 jieba 分词。
    pub fn search(&self, query: &str, top: Option<usize>) -> Result<Vec<SearchHit>, rusqlite::Error> {
        // FTS5 phrase: 双引号包裹,内部双引号用 "" 转义
        let escaped = query.replace('"', "\"\"");
        let fts_query = format!("\"{}\"", escaped);

        // d.status = 'validated' 是防御性过滤:upsert_doc_in_tx 已保证 docs 表只含 validated 行
        let fts_sql = "SELECT d.path, d.name, d.summary, bm25(docs_fts, 1.0, 3.0, 3.0, 2.0, 1.0, 3.0) AS rank
                       FROM docs_fts
                       JOIN docs d ON d.rowid = docs_fts.rowid
                       WHERE docs_fts MATCH ?1 AND d.status = 'validated'
                       ORDER BY rank";
        let mut stmt = self.conn.prepare(fts_sql)?;
        let rows = stmt.query_map(params![fts_query], |row| {
            Ok(SearchHit {
                path: row.get(0)?,
                name: row.get(1)?,
                summary: row.get(2)?,
                rank: row.get(3)?,
            })
        })?;
        let cap = top.unwrap_or(64);
        let mut hits: Vec<SearchHit> = Vec::with_capacity(cap);
        for row in rows {
            hits.push(row?);
        }

        if let Some(n) = top {
            hits.truncate(n);
        }
        Ok(hits)
    }

    /// 列出所有 tag 及其文档列表。
    pub fn list_tags(&self) -> Result<BTreeMap<String, Vec<String>>, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT doc_path, tag FROM tags ORDER BY tag, doc_path")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for row in rows {
            let (doc, tag): (String, String) = row?;
            map.entry(tag).or_default().push(doc);
        }
        for docs in map.values_mut() {
            docs.sort();
        }
        Ok(map)
    }

    /// 查询指定 tag 的文档。
    pub fn docs_for_tag(&self, tag: &str) -> Result<Vec<String>, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT doc_path FROM tags WHERE tag = ?1 ORDER BY doc_path")?;
        let rows = stmt.query_map(params![tag], |row| row.get::<_, String>(0))?;
        let mut docs: Vec<String> = Vec::new();
        for row in rows {
            docs.push(row?);
        }
        Ok(docs)
    }

    /// 文档的正向链接。
    pub fn outlinks(&self, doc: &str) -> Result<Vec<(String, Option<String>)>, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT target, relation FROM links WHERE source = ?1 ORDER BY target")?;
        let rows = stmt.query_map(params![doc], |row| {
            let relation: Option<String> = row.get(1)?;
            Ok((row.get::<_, String>(0)?, relation.filter(|s| !s.is_empty())))
        })?;
        let mut out: Vec<(String, Option<String>)> = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// 文档的反向链接。
    pub fn inlinks(&self, doc: &str) -> Result<Vec<(String, Option<String>)>, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT source, relation FROM links WHERE target = ?1 ORDER BY source")?;
        let rows = stmt.query_map(params![doc], |row| {
            let relation: Option<String> = row.get(1)?;
            Ok((row.get::<_, String>(0)?, relation.filter(|s| !s.is_empty())))
        })?;
        let mut inn: Vec<(String, Option<String>)> = Vec::new();
        for row in rows {
            inn.push(row?);
        }
        Ok(inn)
    }

    /// 全部链接。
    pub fn all_links(&self) -> Result<Vec<(String, String, Option<String>)>, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT source, target, relation FROM links ORDER BY source, target")?;
        let rows = stmt.query_map([], |row| {
            let relation: Option<String> = row.get(2)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                relation.filter(|s| !s.is_empty()),
            ))
        })?;
        let mut links: Vec<(String, String, Option<String>)> = Vec::new();
        for row in rows {
            links.push(row?);
        }
        Ok(links)
    }

    /// 全部文档元数据。
    pub fn all_docs_meta(&self) -> Result<Vec<DocMeta>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT path, name, summary, domain, has_frontmatter, status,
                    mtime_secs, mtime_nanos, size_bytes
             FROM docs ORDER BY path",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(DocMeta {
                path: row.get(0)?,
                name: row.get(1)?,
                summary: row.get(2)?,
                domain: row.get(3)?,
                has_frontmatter: row.get::<_, i64>(4)? != 0,
                status: row.get::<_, String>(5)?,
                tags: Vec::new(),
                outlinks: Vec::new(),
                mtime_secs: row.get(6)?,
                mtime_nanos: row.get::<_, u32>(7)?,
                size_bytes: row.get::<_, i64>(8)? as u64,
            })
        })?;

        let mut docs: Vec<DocMeta> = Vec::new();
        for row in rows {
            docs.push(row?);
        }

        // 批量填充 tags 和 outlinks
        let mut tags_map: HashMap<String, Vec<String>> = HashMap::new();
        {
            let mut stmt = self.conn.prepare("SELECT doc_path, tag FROM tags")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for row in rows {
                let (doc, tag) = row?;
                tags_map.entry(doc).or_default().push(tag);
            }
        }

        let mut outlinks_map: HashMap<String, Vec<(String, Option<String>)>> = HashMap::new();
        {
            let mut stmt = self
                .conn
                .prepare("SELECT source, target, relation FROM links")?;
            let rows = stmt.query_map([], |row| {
                let relation: Option<String> = row.get(2)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    relation.filter(|s| !s.is_empty()),
                ))
            })?;
            for row in rows {
                let (source, target, relation) = row?;
                outlinks_map
                    .entry(source)
                    .or_default()
                    .push((target, relation));
            }
        }

        for doc in &mut docs {
            if let Some(tags) = tags_map.get(&doc.path) {
                doc.tags = tags.clone();
            }
            if let Some(links) = outlinks_map.get(&doc.path) {
                doc.outlinks = links.clone();
            }
        }

        Ok(docs)
    }

    /// 索引状态(自动修复前快照)。
    pub fn status(&self, kb_root_abs: &str) -> Result<IndexStatus, rusqlite::Error> {
        let index_exists = self.index_path.exists();
        let indexed_count: usize = self
            .conn
            .query_row("SELECT COUNT(*) FROM docs", [], |row| row.get::<_, usize>(0))?;
        let files = scan_files(kb_root_abs);
        let filesystem_count = files.len();

        let mut stale_count = 0;
        let mut missing_count = 0;
        let mut new_count = 0;
        let indexed: HashSet<String> = {
            let mut stmt = self.conn.prepare("SELECT path FROM docs")?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
            rows.filter_map(|r| r.ok()).collect()
        };

        let on_disk: HashSet<String> = files.iter().cloned().collect();

        for path in &indexed {
            if !on_disk.contains(path) {
                missing_count += 1;
            } else if let Some((secs, nanos, size)) = file_stat(kb_root_abs, path) {
                let stored: Option<(i64, u32, i64)> = self
                    .conn
                    .query_row(
                        "SELECT mtime_secs, mtime_nanos, size_bytes FROM docs WHERE path = ?1",
                        params![path],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .optional()?;
                if let Some((s_secs, s_nanos, s_size)) = stored {
                    if s_secs != secs || s_nanos != nanos || s_size != size as i64 {
                        stale_count += 1;
                    }
                }
            }
        }

        for path in &on_disk {
            if !indexed.contains(path) {
                new_count += 1;
            }
        }

        Ok(IndexStatus {
            index_exists,
            index_file: self.index_path.to_string_lossy().to_string(),
            indexed_count,
            filesystem_count,
            stale_count,
            missing_count,
            new_count,
        })
    }

    /// 内部连接(测试用)。
    #[allow(dead_code)]
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

/// 获取单个文件的 mtime/size。
fn file_stat(kb_root_abs: &str, rel_path: &str) -> Option<(i64, u32, u64)> {
    let abs = Path::new(kb_root_abs).join(rel_path.replace('/', std::path::MAIN_SEPARATOR_STR));
    let meta = std::fs::metadata(&abs).ok()?;
    let modified = meta.modified().ok()?;
    let duration = modified.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some((duration.as_secs() as i64, duration.subsec_nanos(), meta.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_kb() -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().to_string();
        (dir, root)
    }

    fn write_doc(root: &str, rel: &str, content: &str) {
        let abs = Path::new(root).join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut f = std::fs::File::create(&abs).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn test_open_and_rebuild() {
        let (_dir, root) = tmp_kb();
        write_doc(
            &root,
            "zoloz/zoloz.md",
            "---\nname: zoloz\nsummary: zoloz summary\ntags: [a, b]\nstatus: validated\n---\nbody\n[[`test/test.md`|rel]]",
        );
        write_doc(
            &root,
            "test/test.md",
            "---\nname: test\nsummary: test summary\ntags: [a]\nstatus: validated\n---\nbody",
        );

        let mut db = IndexDb::open(&root).unwrap();
        let stats = db.full_rebuild(&root).unwrap();
        assert_eq!(stats.indexed, 2);

        let hits = db.search("zoloz", None).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].path, "zoloz/zoloz.md");

        let tags = db.list_tags().unwrap();
        assert!(tags.contains_key("a"));
        assert!(tags.contains_key("b"));

        let out = db.outlinks("zoloz/zoloz.md").unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, "test/test.md");
    }

    #[test]
    fn test_repair_stale() {
        let (_dir, root) = tmp_kb();
        write_doc(
            &root,
            "a.md",
            "---\nname: a\nsummary: a\ntags: []\nstatus: validated\n---\nbody",
        );
        let mut db = IndexDb::open(&root).unwrap();
        db.full_rebuild(&root).unwrap();

        // 修改文件
        write_doc(
            &root,
            "a.md",
            "---\nname: a\nsummary: updated\ntags: [x]\nstatus: validated\n---\nbody new",
        );
        let repair = db.repair_stale(&root).unwrap();
        assert_eq!(repair.updated, 1);

        let hits = db.search("updated", None).unwrap();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn test_remove_doc() {
        let (_dir, root) = tmp_kb();
        write_doc(
            &root,
            "a.md",
            "---\nname: a\nsummary: a\ntags: []\nstatus: validated\n---\n[[b.md]]",
        );
        write_doc(
            &root,
            "b.md",
            "---\nname: b\nsummary: b\ntags: []\nstatus: validated\n---\nbody",
        );
        let mut db = IndexDb::open(&root).unwrap();
        db.full_rebuild(&root).unwrap();

        db.remove_doc("a.md").unwrap();
        let docs = db.all_docs_meta().unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].path, "b.md");

        let links = db.all_links().unwrap();
        assert!(links.is_empty());
    }

    #[test]
    fn test_upsert_missing_file_errors() {
        // 读文件失败应返回 Err,不再静默 return Ok(())
        let (_dir, root) = tmp_kb();
        let mut db = IndexDb::open(&root).unwrap();
        let result = db.upsert_doc(&root, "nonexistent/doc.md");
        assert!(result.is_err(), "upsert of missing file must error, not silently succeed");
    }

    #[test]
    fn test_repair_stale_prunes_links_to_pending() {
        let (_dir, root) = tmp_kb();
        write_doc(
            &root,
            "a.md",
            "---\nname: a\nsummary: a\ntags: []\nstatus: validated\n---\n[[b.md]]",
        );
        write_doc(
            &root,
            "b.md",
            "---\nname: b\nsummary: b\ntags: []\n---\nb body",
        );
        let mut db = IndexDb::open(&root).unwrap();
        db.full_rebuild(&root).unwrap();

        // full_rebuild 后 A→B 被 prune(B 为 pending 不在索引)
        assert!(db.outlinks("a.md").unwrap().is_empty());

        // 改变 A 的 mtime,A 仍为 validated 仍链接 B
        write_doc(
            &root,
            "a.md",
            "---\nname: a\nsummary: a\ntags: []\nstatus: validated\n---\n[[b.md]]\nupdated",
        );

        db.repair_stale(&root).unwrap();

        // repair_stale 应清理指向 pending B 的 link
        assert!(db.outlinks("a.md").unwrap().is_empty(), "repair_stale 后不应有指向 pending 的 link");
        assert!(db.all_links().unwrap().is_empty(), "links 表应为空");
    }

    #[test]
    fn test_upsert_derives_domain_from_path() {
        let (_dir, root) = tmp_kb();
        // 文档 frontmatter 中没有 domain 字段(旧文档迁移场景)
        write_doc(
            &root,
            "zoloz/pay/invoice.md",
            "---\nname: invoice\nsummary: s\ntags: []\nstatus: validated\n---\nbody",
        );
        let mut db = IndexDb::open(&root).unwrap();
        db.full_rebuild(&root).unwrap();
        // domain 从路径推导 = zoloz/pay(父目录),不依赖 frontmatter
        let docs = db.all_docs_meta().unwrap();
        let doc = docs.iter().find(|d| d.path == "zoloz/pay/invoice.md").unwrap();
        assert_eq!(doc.domain, "zoloz/pay");
        // 顶层文档 domain 为空
        write_doc(
            &root,
            "top.md",
            "---\nname: top\nsummary: s\ntags: []\nstatus: validated\n---\nbody",
        );
        db.full_rebuild(&root).unwrap();
        let docs = db.all_docs_meta().unwrap();
        let top = docs.iter().find(|d| d.path == "top.md").unwrap();
        assert_eq!(top.domain, "");
    }

    #[test]
    fn test_schema_version_is_6() {
        let (_dir, root) = tmp_kb();
        write_doc(&root, "a.md", "---\nname: a\nsummary: s\ntags: []\nstatus: validated\n---\nbody");
        let db = IndexDb::open(&root).unwrap();
        let version: String = db.conn()
            .query_row(
                "SELECT value FROM meta WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, "6");
    }

    #[test]
    fn test_open_migrates_v5_and_rebuilds() {
        let (_dir, root) = tmp_kb();
        write_doc(&root, "zoloz/zoloz.md", "---\nname: zoloz\nsummary: s\ntags: []\nstatus: validated\n---\nbody");
        // 先用正常方式建 v6 库并索引
        {
            let mut db = IndexDb::open(&root).unwrap();
            db.full_rebuild(&root).unwrap();
        }
        // 模拟 v5 库:改 schema_version 为 5 + docs 表改回 category 列
        {
            let conn = rusqlite::Connection::open(Path::new(&root).join(".akb_index.sqlite")).unwrap();
            conn.execute("UPDATE meta SET value='5' WHERE key='schema_version'", []).unwrap();
            conn.execute("ALTER TABLE docs RENAME TO docs_v6", []).unwrap();
            conn.execute(
                "CREATE TABLE docs (path TEXT PRIMARY KEY, name TEXT NOT NULL, summary TEXT NOT NULL, heading TEXT NOT NULL, body TEXT NOT NULL, tags_text TEXT NOT NULL, category TEXT NOT NULL, has_frontmatter INTEGER NOT NULL, status TEXT NOT NULL DEFAULT 'pending', mtime_secs INTEGER NOT NULL, mtime_nanos INTEGER NOT NULL, size_bytes INTEGER NOT NULL)",
                [],
            ).unwrap();
            conn.execute("INSERT INTO docs SELECT path, name, summary, heading, body, tags_text, '', has_frontmatter, status, mtime_secs, mtime_nanos, size_bytes FROM docs_v6", []).unwrap();
            conn.execute("DROP TABLE docs_v6", []).unwrap();
        }
        // 用新代码 open:应自动迁移到 v6 并重建索引
        let db = IndexDb::open(&root).unwrap();
        let version: String = db.conn()
            .query_row("SELECT value FROM meta WHERE key = 'schema_version'", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, "6");
        // 索引已重建
        let docs = db.all_docs_meta().unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].path, "zoloz/zoloz.md");
        assert_eq!(docs[0].domain, "zoloz");
    }
}
