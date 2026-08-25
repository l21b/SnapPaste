use crate::db::core::get_conn;
use crate::db::queries::{get_records, get_records_with_conn, get_settings, save_settings};
use crate::models::{
    ContentType, FavoriteExportResult, FavoriteTransferItem, FavoriteTransferPackage,
};
use chrono::Local;
use rusqlite::{Connection, params};
use std::collections::HashMap;
use std::path::PathBuf;

const MAX_IMPORT_FILE_BYTES: u64 = 10 * 1024 * 1024;

fn validate_import_path(path: &str) -> Result<std::fs::Metadata, String> {
    if path.trim().is_empty() {
        return Err("导入路径不能为空".to_string());
    }

    let metadata = std::fs::metadata(path).map_err(|e| format!("无法访问文件 {}: {}", path, e))?;
    if !metadata.is_file() {
        return Err(format!("导入路径不是文件: {}", path));
    }
    if metadata.len() > MAX_IMPORT_FILE_BYTES {
        return Err(format!(
            "导入文件过大（最大允许 {} MiB）",
            MAX_IMPORT_FILE_BYTES / 1024 / 1024
        ));
    }

    Ok(metadata)
}

/// 导出逻辑：将当前收藏和设置打包为 JSON 结构
pub fn collect_favorites_package() -> Result<FavoriteTransferPackage, String> {
    let records = get_records(Some(true), None, None).map_err(|e| e.to_string())?;
    let favorites = records
        .into_iter()
        .filter(|r| r.content_type != ContentType::Image)
        .map(|r| FavoriteTransferItem {
            content_type: r.content_type,
            content: r.content,
            is_pinned: r.is_pinned,
        })
        .collect();

    let settings = redact_settings_for_export(get_settings().map_err(|e| e.to_string())?);
    Ok(FavoriteTransferPackage {
        favorites,
        settings,
    })
}

fn redact_settings_for_export(mut settings: crate::models::Settings) -> crate::models::Settings {
    settings.ai_api_key.clear();
    settings
}

fn preserve_local_secrets(
    mut imported: crate::models::Settings,
) -> Result<crate::models::Settings, String> {
    if imported.ai_api_key.trim().is_empty() {
        imported.ai_api_key = get_settings().map_err(|e| e.to_string())?.ai_api_key;
    }
    Ok(imported)
}

/// 导出到文件
pub fn export_favorites_to_path_logic(path: String) -> Result<FavoriteExportResult, String> {
    let mut output = PathBuf::from(path.trim());
    if output.as_os_str().is_empty() {
        return Err("path is empty".to_string());
    }

    if output.is_dir() {
        output.push(format!(
            "snappaste-favorites-{}.json",
            Local::now().format("%Y%m%d-%H%M%S")
        ));
    } else if output.extension().is_none() {
        output.set_extension("json");
    }

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let pkg = collect_favorites_package()?;
    let count = pkg.favorites.len() as i32;
    let json = serde_json::to_string_pretty(&pkg).map_err(|e| e.to_string())?;

    std::fs::write(&output, json).map_err(|e| e.to_string())?;

    Ok(FavoriteExportResult {
        count,
        path: output.to_string_lossy().to_string(),
    })
}

fn import_favorite_items(items: Vec<FavoriteTransferItem>) -> Result<i32, String> {
    let mut conn = get_conn().map_err(|e| e.to_string())?;
    import_favorite_items_with_conn(&mut conn, items)
}

fn import_favorite_items_with_conn(
    conn: &mut Connection,
    items: Vec<FavoriteTransferItem>,
) -> Result<i32, String> {
    let existing_records =
        get_records_with_conn(conn, None, None, None).map_err(|e| e.to_string())?;
    let mut existing: HashMap<(ContentType, String), (i64, bool, bool)> = existing_records
        .into_iter()
        .filter(|record| record.content_type != ContentType::Image)
        .map(|record| {
            (
                (record.content_type, record.content.trim().to_string()),
                (record.id, record.is_favorite, record.is_pinned),
            )
        })
        .collect();

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let created_at = Local::now().to_rfc3339();
    let mut imported_count = 0;

    for item in items {
        let content = item.content.trim().to_string();
        if content.is_empty() || item.content_type == ContentType::Image {
            continue;
        }

        let key = (item.content_type.clone(), content.clone());
        if let Some((id, is_favorite, is_pinned)) = existing.get_mut(&key) {
            let merged_pinned = *is_pinned || item.is_pinned;
            if !*is_favorite || merged_pinned != *is_pinned {
                tx.execute(
                    "UPDATE clipboard_history SET is_favorite = 1, is_pinned = ?1 WHERE id = ?2",
                    params![merged_pinned as i32, *id],
                )
                .map_err(|e| format!("更新已有收藏失败: {e}"))?;
            }
            if !*is_favorite {
                imported_count += 1;
            }
            *is_favorite = true;
            *is_pinned = merged_pinned;
        } else {
            tx.execute(
                "INSERT INTO clipboard_history (content_type, content, image_data, is_favorite, is_pinned, created_at) VALUES (?1, ?2, NULL, 1, ?3, ?4)",
                params![item.content_type, &content, item.is_pinned as i32, &created_at],
            )
            .map_err(|e| format!("插入失败: {e}"))?;
            let id = tx.last_insert_rowid();
            existing.insert(key, (id, true, item.is_pinned));
            imported_count += 1;
        }
    }
    tx.commit().map_err(|e| format!("提交失败: {}", e))?;

    Ok(imported_count)
}

/// 从 JSON 数据导入收藏和设置（极速批量插入优化版）
#[allow(dead_code)]
pub fn import_favorites_from_payload(payload: &str) -> Result<(i32, bool), String> {
    let parsed: FavoriteTransferPackage =
        serde_json::from_str(payload).map_err(|e| format!("invalid json: {}", e))?;
    let imported_count = import_favorite_items(parsed.favorites)?;

    let settings_imported = if !parsed.settings.hotkey.is_empty() {
        let settings = preserve_local_secrets(parsed.settings)?;
        save_settings(&settings).map_err(|e| e.to_string())?;
        true
    } else {
        false
    };

    Ok((imported_count, settings_imported))
}

/// Slint/native desktop path: synchronously import a validated JSON file.
pub fn import_favorites_from_path_sync(path: &str) -> Result<(i32, bool), String> {
    let path = path.trim();
    validate_import_path(path)?;
    let json =
        std::fs::read_to_string(path).map_err(|error| format!("无法读取文件 {path}: {error}"))?;
    import_favorites_from_payload(&json)
}

#[cfg(test)]
mod tests {
    use super::{import_favorite_items_with_conn, redact_settings_for_export};
    use crate::db::core::initialize_schema;
    use crate::db::queries::get_records_with_conn;
    use crate::models::{ContentType, FavoriteTransferItem, Settings};
    use rusqlite::Connection;

    #[test]
    fn exported_settings_do_not_include_api_key() {
        let settings = Settings {
            ai_api_key: "secret-key".to_string(),
            ..Settings::default()
        };

        assert!(redact_settings_for_export(settings).ai_api_key.is_empty());
    }

    #[test]
    fn favorite_import_is_atomic_when_an_insert_fails() {
        let mut conn = Connection::open_in_memory().expect("open in-memory database");
        initialize_schema(&conn).expect("initialize test schema");
        conn.execute_batch(
            "CREATE TRIGGER reject_broken_favorite
             BEFORE INSERT ON clipboard_history
             WHEN NEW.content = 'broken'
             BEGIN
                 SELECT RAISE(ABORT, 'rejected by test trigger');
             END;",
        )
        .expect("create failure trigger");

        let result = import_favorite_items_with_conn(
            &mut conn,
            vec![
                FavoriteTransferItem {
                    content_type: ContentType::Text,
                    content: "valid".to_string(),
                    is_pinned: false,
                },
                FavoriteTransferItem {
                    content_type: ContentType::Text,
                    content: "broken".to_string(),
                    is_pinned: false,
                },
            ],
        );

        assert!(result.is_err());
        let records = get_records_with_conn(&conn, Some(true), None, None).expect("read favorites");
        assert!(records.is_empty());
    }

    #[test]
    fn favorite_import_merges_existing_records_and_ignores_invalid_duplicates() {
        let mut conn = Connection::open_in_memory().expect("open in-memory database");
        initialize_schema(&conn).expect("initialize test schema");
        conn.execute(
            "INSERT INTO clipboard_history (content_type, content, is_favorite, is_pinned, created_at)
             VALUES ('text', 'existing', 1, 0, '2026-01-01T00:00:00Z')",
            [],
        )
        .expect("insert existing favorite");
        conn.execute(
            "INSERT INTO clipboard_history (content_type, content, is_favorite, is_pinned, created_at)
             VALUES ('text', 'promote-me', 0, 0, '2026-01-02T00:00:00Z')",
            [],
        )
        .expect("insert existing history record");

        let imported = import_favorite_items_with_conn(
            &mut conn,
            vec![
                FavoriteTransferItem {
                    content_type: ContentType::Text,
                    content: "existing".to_string(),
                    is_pinned: false,
                },
                FavoriteTransferItem {
                    content_type: ContentType::Text,
                    content: " new ".to_string(),
                    is_pinned: true,
                },
                FavoriteTransferItem {
                    content_type: ContentType::Text,
                    content: "promote-me".to_string(),
                    is_pinned: true,
                },
                FavoriteTransferItem {
                    content_type: ContentType::Text,
                    content: "new".to_string(),
                    is_pinned: false,
                },
                FavoriteTransferItem {
                    content_type: ContentType::Text,
                    content: "   ".to_string(),
                    is_pinned: false,
                },
                FavoriteTransferItem {
                    content_type: ContentType::Image,
                    content: "ignored-image".to_string(),
                    is_pinned: false,
                },
            ],
        )
        .expect("import favorites");

        let records = get_records_with_conn(&conn, Some(true), None, None).expect("read favorites");
        assert_eq!(imported, 2);
        assert_eq!(records.len(), 3);
        let new_record = records
            .iter()
            .find(|record| record.content == "new")
            .expect("find imported favorite");
        assert!(new_record.is_pinned);
        let promoted = records
            .iter()
            .find(|record| record.content == "promote-me")
            .expect("find promoted favorite");
        assert!(promoted.is_favorite);
        assert!(promoted.is_pinned);
    }
}
