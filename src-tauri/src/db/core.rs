use crate::models::Settings;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use std::path::PathBuf;
use std::sync::OnceLock;

const DB_FILE: &str = "snappaste.db";

// 🚀 核心升级 1：使用 r2d2 连接池替代单例 Mutex，彻底解放多线程并发能力
static DB_POOL: OnceLock<Pool<SqliteConnectionManager>> = OnceLock::new();

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
pub fn get_conn() -> r2d2::PooledConnection<SqliteConnectionManager> {
    DB_POOL
        .get()
        .expect("数据库连接池未初始化")
        .get()
        .expect("无法从连接池获取连接")
}

/// 初始化数据库（建表、初始化设置）
pub fn init_database() -> Result<(), Box<dyn std::error::Error>> {
    let path = get_db_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
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
    DB_POOL.set(pool).map_err(|_| "数据库连接池重复初始化")?;

    let conn = get_conn();

    // 🚀 核心升级 2：极简 Schema。配置项全部作为 JSON 存储在 config 字段。
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

    // 初始化默认设置 (如果尚未存在)
    let default_settings = Settings::default();
    let json_str = serde_json::to_string(&default_settings)?;
    conn.execute(
        "INSERT OR IGNORE INTO app_settings (id, config) VALUES (1, ?1)",
        params![json_str],
    )?;

    Ok(())
}
