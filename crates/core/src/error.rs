//! error.rs - 统一错误类型。

use std::io;

/// 知识库操作统一错误。
///
/// 正常路径返回 Ok,错误路径用 ? 传播;main 统一转 JSON 输出。
#[derive(Debug, thiserror::Error)]
pub enum KbError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("{0}")]
    Other(String),
}
