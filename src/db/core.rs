use crate::models::Settings;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::{error::Error, fmt};

const DB_FILE: &str = "snappaste.db";

// 🚀 核心升级 1：使用 r2d2 连接池替代单例 Mutex，彻底解放多线程并发能力
static DB_POOL: OnceLock<Pool<SqliteConnectionManager>> = OnceLock::new();

#[derive(Debug)]
pub enum DbError {
    NotInitialized,
    AlreadyInitialized,
    Pool(r2d2::Error),
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInitialized => write!(f, "数据库连接池未初始化"),
            Self::AlreadyInitialized => write!(f, "数据库连接池重复初始化"),
            Self::Pool(error) => write!(f, "无法从数据库连接池获取连接: {error}"),
            Self::Sqlite(error) => write!(f, "SQLite 错误: {error}"),
            Self::Io(error) => write!(f, "数据库文件错误: {error}"),
            Self::Json(error) => write!(f, "数据库配置 JSON 错误: {error}"),
        }
    }
}

impl Error for DbError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Pool(error) => Some(error),
            Self::Sqlite(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::NotInitialized | Self::AlreadyInitialized => None,
        }
    }
}

impl From<r2d2::Error> for DbError {
    fn from(error: r2d2::Error) -> Self {
        Self::Pool(error)
    }
}

impl From<rusqlite::Error> for DbError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl From<std::io::Error> for DbError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for DbError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

pub type DbResult<T> = Result<T, DbError>;

#[cfg(target_os = "windows")]
fn preferred_windows_db_path() -> Option<PathBuf> {
    std::env::var("LOCALAPPDATA")
        .ok()
        .map(|data_dir| PathBuf::from(data_dir).join("SnapPaste").join(DB_FILE))
}

#[cfg(target_os = "windows")]
fn get_db_path() -> PathBuf {
    if let Some(path) = preferred_windows_db_path() {
        return path;
    }
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.push(DB_FILE);
    path
}

#[cfg(not(target_os = "windows"))]
fn get_db_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.push(DB_FILE);
    path
}

/// 从连接池获取一个可用连接
pub fn get_conn() -> DbResult<r2d2::PooledConnection<SqliteConnectionManager>> {
    DB_POOL
        .get()
        .ok_or(DbError::NotInitialized)?
        .get()
        .map_err(DbError::Pool)
}

pub(crate) fn initialize_schema(conn: &rusqlite::Connection) -> DbResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS clipboard_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content_type TEXT NOT NULL,
            content TEXT,
            image_data BLOB,
            is_favorite INTEGER DEFAULT 0,
            is_pinned INTEGER DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS app_settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            config TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS window_state (
            label TEXT PRIMARY KEY,
            width INTEGER,
            height INTEGER
        );
        CREATE INDEX IF NOT EXISTS idx_created_at ON clipboard_history(created_at DESC);",
    )?;

    let json = serde_json::to_string(&Settings::default())?;
    conn.execute(
        "INSERT OR IGNORE INTO app_settings (id, config) VALUES (1, ?1)",
        params![json],
    )?;

    Ok(())
}

/// 初始化数据库（建表、初始化设置）
pub fn init_database() -> DbResult<()> {
    let path = get_db_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // 配置 SQLite 连接管理器并开启高性能 WAL 模式
    let manager = SqliteConnectionManager::file(path).with_init(|c| {
        c.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA busy_timeout=5000;", // 防止被其他线程锁住时的瞬间报错
        )
    });

    let pool = Pool::builder().max_size(5).build(manager)?;
    DB_POOL.set(pool).map_err(|_| DbError::AlreadyInitialized)?;

    let conn = get_conn()?;
    initialize_schema(&conn)
}

#[cfg(test)]
mod tests {
    use super::{DbError, get_conn};

    #[test]
    fn get_conn_before_initialization_returns_an_error() {
        assert!(matches!(get_conn(), Err(DbError::NotInitialized)));
    }
}
