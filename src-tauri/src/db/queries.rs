use crate::db::core::{DbResult, get_conn};
use crate::models::{ClipboardRecord, ContentType, Settings};
use chrono::Local;
use rusqlite::{Connection, OptionalExtension, Row, params};

const MAX_PAGE_SIZE: i32 = 500;

// =============================================================================
// 数据映射助手
// =============================================================================

fn map_record_row(row: &Row<'_>) -> Result<ClipboardRecord, rusqlite::Error> {
    Ok(ClipboardRecord {
        id: row.get(0)?,
        content_type: row.get(1)?,
        content: row.get(2)?,
        image_data: row.get(3)?,
        is_favorite: row.get::<_, i32>(4)? > 0,
        is_pinned: row.get::<_, i32>(5)? > 0,
        created_at: row.get(6)?,
    })
}

fn escape_like_pattern(keyword: &str) -> String {
    let escaped = keyword
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{}%", escaped)
}

fn normalize_limit(limit: i32) -> i32 {
    limit.clamp(0, MAX_PAGE_SIZE)
}

fn normalize_offset(offset: i32) -> i32 {
    offset.max(0)
}

fn deserialize_settings(json: &str) -> DbResult<Settings> {
    Ok(serde_json::from_str(json)?)
}

fn serialize_settings(settings: &Settings) -> DbResult<String> {
    Ok(serde_json::to_string(settings)?)
}

// =============================================================================
// 设置管理与清理策略
// =============================================================================

/// 获取应用设置：直接解析 JSON
pub fn get_settings() -> DbResult<Settings> {
    let conn = get_conn()?;
    get_settings_with_conn(&conn)
}

pub(crate) fn get_settings_with_conn(conn: &Connection) -> DbResult<Settings> {
    let json = conn
        .query_row("SELECT config FROM app_settings WHERE id = 1", [], |row| {
            row.get::<_, String>(0)
        })
        .optional()?;

    json.as_deref()
        .map(deserialize_settings)
        .unwrap_or_else(|| Ok(Settings::default()))
}

/// 保存应用设置
pub fn save_settings(settings: &Settings) -> DbResult<()> {
    let mut conn = get_conn()?;
    let tx = conn.transaction()?;
    save_settings_with_conn(&tx, settings)?;
    tx.commit()?;
    Ok(())
}

pub(crate) fn save_settings_with_conn(conn: &Connection, settings: &Settings) -> DbResult<()> {
    let json_str = serialize_settings(settings)?;

    conn.execute(
        "UPDATE app_settings SET config = ?1 WHERE id = 1",
        params![json_str],
    )?;

    apply_retention_policy(conn, settings)?;
    Ok(())
}

fn apply_retention_policy(conn: &rusqlite::Connection, settings: &Settings) -> DbResult<()> {
    let keep_days = settings.keep_days.max(0);
    let max_records = settings.max_records.max(0);

    if keep_days > 0 {
        conn.execute(
            "DELETE FROM clipboard_history WHERE COALESCE(is_favorite, 0) = 0 AND julianday(created_at) < julianday('now', ?1)",
            params![format!("-{} days", keep_days)],
        )?;
    }
    if max_records > 0 {
        conn.execute(
            "DELETE FROM clipboard_history WHERE COALESCE(is_favorite, 0) = 0 AND id NOT IN (SELECT id FROM clipboard_history WHERE COALESCE(is_favorite, 0) = 0 ORDER BY created_at DESC, id DESC LIMIT ?1)",
            params![max_records],
        )?;
    }
    Ok(())
}

// =============================================================================
// 历史记录基础 CRUD
// =============================================================================

const SELECT_NO_IMAGE_PREFIX: &str = "SELECT id, content_type, COALESCE(content, '') as content, NULL as image_data, COALESCE(is_favorite, 0) as is_favorite, COALESCE(is_pinned, 0) as is_pinned, created_at FROM clipboard_history";

pub fn get_records(
    is_favorite: Option<bool>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> DbResult<Vec<ClipboardRecord>> {
    let conn = get_conn()?;
    get_records_with_conn(&conn, is_favorite, limit, offset)
}

pub(crate) fn get_records_with_conn(
    conn: &Connection,
    is_favorite: Option<bool>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> DbResult<Vec<ClipboardRecord>> {
    let limit = limit.map(normalize_limit);
    let offset = normalize_offset(offset.unwrap_or(0));
    let mut sql = SELECT_NO_IMAGE_PREFIX.to_string();
    if let Some(fav) = is_favorite {
        sql.push_str(&format!(" WHERE COALESCE(is_favorite, 0) = {}", fav as i32));
    }
    sql.push_str(" ORDER BY COALESCE(is_pinned, 0) DESC, created_at DESC, id DESC");
    if limit.is_some() {
        sql.push_str(" LIMIT ?1 OFFSET ?2");
    }

    let mut stmt = conn.prepare_cached(&sql)?;
    let records: Vec<ClipboardRecord> = match limit {
        Some(limit) => stmt
            .query_map(params![limit, offset], map_record_row)?
            .collect::<Result<_, _>>()?,
        None => stmt
            .query_map([], map_record_row)?
            .collect::<Result<_, _>>()?,
    };
    Ok(records)
}

pub fn search_records(
    keyword: &str,
    is_favorite: Option<bool>,
    limit: i32,
) -> DbResult<Vec<ClipboardRecord>> {
    let conn = get_conn()?;
    search_records_with_conn(&conn, keyword, is_favorite, limit)
}

fn search_records_with_conn(
    conn: &Connection,
    keyword: &str,
    is_favorite: Option<bool>,
    limit: i32,
) -> DbResult<Vec<ClipboardRecord>> {
    let pattern = escape_like_pattern(keyword);
    let limit = normalize_limit(limit);
    let mut sql = format!(
        "{} WHERE content LIKE ?1 ESCAPE '\\'",
        SELECT_NO_IMAGE_PREFIX
    );
    if let Some(fav) = is_favorite {
        sql.push_str(&format!(" AND COALESCE(is_favorite, 0) = {}", fav as i32));
    }
    sql.push_str(" ORDER BY COALESCE(is_pinned, 0) DESC, created_at DESC, id DESC LIMIT ?2");

    let mut stmt = conn.prepare_cached(&sql)?;
    let records: Vec<ClipboardRecord> = stmt
        .query_map(params![pattern, limit], map_record_row)?
        .collect::<Result<_, _>>()?;
    Ok(records)
}

/// 🚀 核心升级 3：基于事务的去重机制，杜绝数据损坏
pub fn add_record(record: ClipboardRecord) -> DbResult<i64> {
    let mut conn = get_conn()?;
    add_record_with_conn(&mut conn, record)
}

pub(crate) fn add_record_with_conn(
    conn: &mut Connection,
    record: ClipboardRecord,
) -> DbResult<i64> {
    let settings = get_settings_with_conn(conn)?;
    let is_dedup_target =
        record.content_type != ContentType::Image && !record.content.trim().is_empty();
    let mut insert_id = 0;

    // 开启数据库事务，确保接下来的多步操作具有原子性（要么全成功，要么全失败回滚）
    let tx = conn.transaction()?;

    if is_dedup_target {
        let existing = tx.query_row(
            "SELECT id, COALESCE(is_favorite, 0), COALESCE(is_pinned, 0) 
             FROM clipboard_history 
             WHERE content_type = ?1 AND content = ?2 
             ORDER BY COALESCE(is_favorite, 0) DESC, COALESCE(is_pinned, 0) DESC, created_at DESC LIMIT 1",
            params![&record.content_type, &record.content],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i32>(1)?, row.get::<_, i32>(2)?)),
        ).optional()?;

        if let Some((keep_id, keep_favorite, keep_pinned)) = existing {
            let merged_favorite = keep_favorite > 0 || record.is_favorite;
            let merged_pinned = keep_pinned > 0 || record.is_pinned;

            tx.execute(
                "UPDATE clipboard_history SET created_at = ?1, is_favorite = ?2, is_pinned = ?3 WHERE id = ?4",
                params![&record.created_at, merged_favorite as i32, merged_pinned as i32, keep_id],
            )?;

            tx.execute(
                "DELETE FROM clipboard_history WHERE content_type = ?1 AND content = ?2 AND id <> ?3",
                params![&record.content_type, &record.content, keep_id],
            )?;

            insert_id = keep_id;
        }
    }

    if insert_id == 0 {
        tx.execute(
            "INSERT INTO clipboard_history (content_type, content, image_data, is_favorite, is_pinned, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![record.content_type, record.content, record.image_data, record.is_favorite as i32, record.is_pinned as i32, record.created_at]
        )?;
        insert_id = tx.last_insert_rowid();
    }

    apply_retention_policy(&tx, &settings)?;
    tx.commit()?;

    Ok(insert_id)
}

pub fn get_record_by_id(id: i64) -> DbResult<Option<ClipboardRecord>> {
    let conn = get_conn()?;
    let mut stmt = conn.prepare_cached(
        "SELECT id, content_type, COALESCE(content, '') as content, image_data, COALESCE(is_favorite, 0) as is_favorite, COALESCE(is_pinned, 0) as is_pinned, created_at 
         FROM clipboard_history WHERE id = ?1 LIMIT 1"
    )?;
    Ok(stmt.query_row(params![id], map_record_row).optional()?)
}

pub fn delete_record(id: i64) -> DbResult<usize> {
    Ok(get_conn()?.execute("DELETE FROM clipboard_history WHERE id = ?1", params![id])?)
}

pub fn clear_records(is_favorite: Option<bool>) -> DbResult<usize> {
    let conn = get_conn()?;
    Ok(match is_favorite {
        Some(true) => conn.execute(
            "DELETE FROM clipboard_history WHERE COALESCE(is_favorite, 0) = 1",
            [],
        ),
        Some(false) => conn.execute(
            "DELETE FROM clipboard_history WHERE COALESCE(is_favorite, 0) = 0",
            [],
        ),
        None => conn.execute("DELETE FROM clipboard_history", []),
    }?)
}

pub fn update_record_status(
    id: i64,
    is_favorite: Option<bool>,
    is_pinned: Option<bool>,
) -> DbResult<()> {
    let conn = get_conn()?;
    if let Some(fav) = is_favorite {
        conn.execute(
            "UPDATE clipboard_history SET is_favorite = ?1 WHERE id = ?2",
            params![fav as i32, id],
        )?;
    }
    if let Some(pin) = is_pinned {
        conn.execute(
            "UPDATE clipboard_history SET is_pinned = ?1 WHERE id = ?2",
            params![pin as i32, id],
        )?;
    }
    Ok(())
}

// =============================================================================
// 高阶封装层：向外提供统一签名的业务管道
// =============================================================================

pub fn get_history(limit: i32, offset: i32) -> Result<Vec<ClipboardRecord>, String> {
    get_records(None, Some(limit), Some(offset)).map_err(|e| e.to_string())
}

pub fn search_history(keyword: &str, limit: i32) -> Result<Vec<ClipboardRecord>, String> {
    search_records(keyword, None, limit).map_err(|e| e.to_string())
}

pub fn get_favorites(limit: i32, offset: i32) -> Result<Vec<ClipboardRecord>, String> {
    get_records(Some(true), Some(limit), Some(offset)).map_err(|e| e.to_string())
}

pub fn search_favorites(keyword: &str, limit: i32) -> Result<Vec<ClipboardRecord>, String> {
    search_records(keyword, Some(true), limit).map_err(|e| e.to_string())
}

pub fn delete_item(id: i64) -> Result<usize, String> {
    delete_record(id).map_err(|e| e.to_string())
}

pub fn clear_history_records() -> Result<usize, String> {
    clear_records(Some(false)).map_err(|e| e.to_string())
}

pub fn clear_favorite_records() -> Result<usize, String> {
    clear_records(Some(true)).map_err(|e| e.to_string())
}

pub fn toggle_favorite(id: i64, is_favorite: bool) -> Result<(), String> {
    update_record_status(id, Some(is_favorite), None).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn toggle_pinned(id: i64, is_pinned: bool) -> Result<(), String> {
    update_record_status(id, None, Some(is_pinned)).map_err(|e| e.to_string())?;
    Ok(())
}

#[allow(dead_code)]
pub fn add_clipboard_record(record: ClipboardRecord) -> Result<i64, String> {
    add_record(record).map_err(|e| e.to_string())
}

// =============================================================================
// 定制化业务逻辑助手
// =============================================================================

/// 将一段内容封装为标准的收藏记录并存入数据库
pub fn add_custom_favorite_record_logic(content: String) -> Result<i64, String> {
    let text = content.trim();
    if text.is_empty() {
        return Err("content cannot be empty".to_string());
    }

    let record = ClipboardRecord {
        id: 0,
        content_type: ContentType::Text,
        content: text.to_string(),
        image_data: None,
        is_favorite: true,
        is_pinned: false,
        created_at: Local::now().to_rfc3339(),
    };

    add_record(record).map_err(|e| e.to_string())
}

/// 获取窗口保存的状态（宽、高）
pub fn get_window_state(label: &str) -> DbResult<Option<(u32, u32)>> {
    let conn = get_conn()?;
    Ok(conn
        .query_row(
            "SELECT width, height FROM window_state WHERE label = ?1",
            [label],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?)
}

/// 保存窗口的状态（宽、高）
pub fn save_window_state(label: &str, width: u32, height: u32) -> DbResult<()> {
    let conn = get_conn()?;
    conn.execute(
        "INSERT INTO window_state (label, width, height) VALUES (?1, ?2, ?3)
         ON CONFLICT(label) DO UPDATE SET width = ?2, height = ?3",
        params![label, width, height],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        add_record_with_conn, deserialize_settings, escape_like_pattern, get_records_with_conn,
        map_record_row, normalize_limit, normalize_offset, save_settings_with_conn,
        search_records_with_conn, serialize_settings,
    };
    use crate::db::core::initialize_schema;
    use crate::models::{ClipboardRecord, ContentType, Settings};
    use rusqlite::Connection;

    fn test_database() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        initialize_schema(&conn).expect("initialize test schema");
        conn
    }

    fn text_record(
        content: &str,
        created_at: &str,
        is_favorite: bool,
        is_pinned: bool,
    ) -> ClipboardRecord {
        ClipboardRecord {
            id: 0,
            content_type: ContentType::Text,
            content: content.to_string(),
            image_data: None,
            is_favorite,
            is_pinned,
            created_at: created_at.to_string(),
        }
    }

    #[test]
    fn escape_like_pattern_escapes_all_special_characters() {
        assert_eq!(escape_like_pattern(r"50%_off\today"), r"%50\%\_off\\today%");
    }

    #[test]
    fn pagination_values_are_bounded() {
        assert_eq!(normalize_limit(-1), 0);
        assert_eq!(normalize_limit(100), 100);
        assert_eq!(normalize_limit(10_000), 500);
        assert_eq!(normalize_offset(-1), 0);
        assert_eq!(normalize_offset(25), 25);
    }

    #[test]
    fn settings_json_round_trip_preserves_values() {
        let settings = Settings::default();
        let json = serialize_settings(&settings).expect("serialize settings");
        let decoded = deserialize_settings(&json).expect("deserialize settings");

        assert_eq!(decoded.hotkey, settings.hotkey);
        assert_eq!(decoded.theme, settings.theme);
        assert_eq!(decoded.max_records, settings.max_records);
    }

    #[test]
    fn malformed_settings_json_is_rejected() {
        assert!(deserialize_settings("{not-json}").is_err());
    }

    #[test]
    fn map_record_row_rejects_invalid_content_type() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        let mut stmt = conn
            .prepare("SELECT 1, 'unknown', 'content', NULL, 0, 0, '2026-01-01'")
            .expect("prepare test query");

        let result = stmt.query_row([], map_record_row);
        assert!(result.is_err());
    }

    #[test]
    fn map_record_row_maps_valid_record() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        let mut stmt = conn
            .prepare("SELECT 7, 'link', 'https://example.com', NULL, 1, 0, '2026-01-01'")
            .expect("prepare test query");

        let record = stmt.query_row([], map_record_row).expect("map record");
        assert_eq!(record.id, 7);
        assert_eq!(record.content_type, ContentType::Link);
        assert!(record.is_favorite);
        assert!(!record.is_pinned);
    }

    #[test]
    fn duplicate_record_keeps_one_row_and_merges_flags() {
        let mut conn = test_database();
        let first_id = add_record_with_conn(
            &mut conn,
            text_record("same", "2026-01-01T00:00:00Z", true, false),
        )
        .expect("insert first record");
        let second_id = add_record_with_conn(
            &mut conn,
            text_record("same", "2026-01-02T00:00:00Z", false, true),
        )
        .expect("merge duplicate record");

        let records = get_records_with_conn(&conn, None, None, None).expect("read records");
        assert_eq!(first_id, second_id);
        assert_eq!(records.len(), 1);
        assert!(records[0].is_favorite);
        assert!(records[0].is_pinned);
        assert_eq!(records[0].created_at, "2026-01-02T00:00:00Z");
    }

    #[test]
    fn max_record_retention_never_removes_favorites() {
        let mut conn = test_database();
        let settings = Settings {
            keep_days: 0,
            max_records: 2,
            ..Settings::default()
        };
        save_settings_with_conn(&conn, &settings).expect("save retention settings");

        add_record_with_conn(
            &mut conn,
            text_record("favorite", "2026-01-01T00:00:00Z", true, false),
        )
        .expect("insert favorite");
        for day in 2..=4 {
            add_record_with_conn(
                &mut conn,
                text_record(
                    &format!("normal-{day}"),
                    &format!("2026-01-0{day}T00:00:00Z"),
                    false,
                    false,
                ),
            )
            .expect("insert normal record");
        }

        let records = get_records_with_conn(&conn, None, None, None).expect("read records");
        let contents: Vec<_> = records
            .iter()
            .map(|record| record.content.as_str())
            .collect();
        assert_eq!(records.len(), 3);
        assert!(contents.contains(&"favorite"));
        assert!(contents.contains(&"normal-4"));
        assert!(contents.contains(&"normal-3"));
        assert!(!contents.contains(&"normal-2"));
    }

    #[test]
    fn age_retention_removes_only_expired_non_favorites() {
        let conn = test_database();
        conn.execute_batch(
            "INSERT INTO clipboard_history (content_type, content, is_favorite, created_at) VALUES
                ('text', 'expired', 0, datetime('now', '-40 days')),
                ('text', 'expired-favorite', 1, datetime('now', '-40 days')),
                ('text', 'recent', 0, datetime('now', '-2 days'));",
        )
        .expect("seed retention records");

        let settings = Settings {
            keep_days: 30,
            max_records: 0,
            ..Settings::default()
        };
        save_settings_with_conn(&conn, &settings).expect("apply age retention");

        let records = get_records_with_conn(&conn, None, None, None).expect("read records");
        let contents: Vec<_> = records
            .iter()
            .map(|record| record.content.as_str())
            .collect();
        assert_eq!(records.len(), 2);
        assert!(contents.contains(&"expired-favorite"));
        assert!(contents.contains(&"recent"));
        assert!(!contents.contains(&"expired"));
    }

    #[test]
    fn pagination_is_stable_and_pinned_records_come_first() {
        let mut conn = test_database();
        let settings = Settings {
            keep_days: 0,
            ..Settings::default()
        };
        save_settings_with_conn(&conn, &settings).expect("disable age retention");

        add_record_with_conn(
            &mut conn,
            text_record("newest", "2026-01-03T00:00:00Z", false, false),
        )
        .expect("insert newest");
        add_record_with_conn(
            &mut conn,
            text_record("old-pinned", "2026-01-01T00:00:00Z", false, true),
        )
        .expect("insert pinned");
        add_record_with_conn(
            &mut conn,
            text_record("middle", "2026-01-02T00:00:00Z", false, false),
        )
        .expect("insert middle");

        let first_page =
            get_records_with_conn(&conn, None, Some(2), Some(0)).expect("read first page");
        let second_page =
            get_records_with_conn(&conn, None, Some(2), Some(2)).expect("read second page");

        assert_eq!(first_page[0].content, "old-pinned");
        assert_eq!(first_page[1].content, "newest");
        assert_eq!(second_page.len(), 1);
        assert_eq!(second_page[0].content, "middle");
    }

    #[test]
    fn equal_timestamps_use_newest_id_as_stable_tiebreaker() {
        let mut conn = test_database();
        let settings = Settings {
            keep_days: 0,
            ..Settings::default()
        };
        save_settings_with_conn(&conn, &settings).expect("disable age retention");

        for content in ["first", "second", "third"] {
            add_record_with_conn(
                &mut conn,
                text_record(content, "2026-01-01T00:00:00Z", false, false),
            )
            .expect("insert record");
        }

        let records = get_records_with_conn(&conn, None, None, None).expect("read records");
        let contents: Vec<_> = records
            .iter()
            .map(|record| record.content.as_str())
            .collect();
        assert_eq!(contents, ["third", "second", "first"]);
    }

    #[test]
    fn search_treats_sql_wildcards_as_literal_text() {
        let mut conn = test_database();
        let settings = Settings {
            keep_days: 0,
            ..Settings::default()
        };
        save_settings_with_conn(&conn, &settings).expect("disable age retention");

        for content in ["save 50%", "save 500", "name_value", "nameXvalue"] {
            add_record_with_conn(
                &mut conn,
                text_record(content, "2026-01-01T00:00:00Z", false, false),
            )
            .expect("insert searchable record");
        }

        let percent = search_records_with_conn(&conn, "50%", None, 50).expect("search percent");
        let underscore =
            search_records_with_conn(&conn, "name_", None, 50).expect("search underscore");
        assert_eq!(percent.len(), 1);
        assert_eq!(percent[0].content, "save 50%");
        assert_eq!(underscore.len(), 1);
        assert_eq!(underscore[0].content, "name_value");
    }
}
