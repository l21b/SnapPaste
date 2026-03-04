use tauri::AppHandle;
use super::CmdResult;
use crate::clipboard::services::paste_record_logic;
use crate::db::{
    add_custom_favorite_record_logic, clear_favorite_records, clear_history_records, delete_item,
    export_favorites_to_path_logic, get_favorites, get_history, import_favorites_from_path_logic,
    search_favorites, search_history, toggle_favorite, toggle_pinned,
};
use crate::models::{ClipboardRecord, FavoriteExportResult};

/// 获取剪贴板历史记录
#[tauri::command]
pub fn get_history_records(limit: i32, offset: i32) -> CmdResult<Vec<ClipboardRecord>> {
    get_history(limit, offset)
}

/// 搜索剪贴板历史记录
#[tauri::command]
pub fn search_records(keyword: String, limit: i32) -> CmdResult<Vec<ClipboardRecord>> {
    search_history(&keyword, limit)
}

/// 获取收藏的记录
#[tauri::command]
pub fn get_favorite_records(limit: i32, offset: i32) -> CmdResult<Vec<ClipboardRecord>> {
    get_favorites(limit, offset)
}

/// 搜索收藏的记录
#[tauri::command]
pub fn search_favorite_records(keyword: String, limit: i32) -> CmdResult<Vec<ClipboardRecord>> {
    search_favorites(&keyword, limit)
}

/// 添加自定义收藏文本
#[tauri::command]
pub fn add_custom_favorite_record(content: String) -> CmdResult<i64> {
    add_custom_favorite_record_logic(content)
}

/// 删除指定记录
#[tauri::command]
pub fn delete_clipboard_record(id: i64) -> CmdResult<()> {
    delete_item(id).map(|_| ())
}

/// 清除非收藏的历史记录
#[tauri::command]
pub fn clear_history_only() -> CmdResult<()> {
    clear_history_records().map(|_| ())
}

/// 清除所有收藏
#[tauri::command]
pub fn clear_favorite_items() -> CmdResult<()> {
    clear_favorite_records().map(|_| ())
}

/// 设置记录的收藏状态
#[tauri::command]
pub fn set_record_favorite_state(id: i64, favorite: bool) -> CmdResult<()> {
    toggle_favorite(id, favorite)
}

/// 设置记录的置顶状态
#[tauri::command]
pub fn set_record_pinned_state(id: i64, pinned: bool) -> CmdResult<()> {
    toggle_pinned(id, pinned)
}

/// 导出收藏到文件
#[tauri::command]
pub fn export_favorites_to_path(path: String) -> CmdResult<FavoriteExportResult> {
    export_favorites_to_path_logic(path)
}

/// 从文件导入收藏和设置
#[tauri::command]
pub async fn import_favorites_from_path(app: tauri::AppHandle, path: String) -> CmdResult<(i32, bool)> {
    import_favorites_from_path_logic(app, path).await
}

/// 粘贴记录指令
#[tauri::command]
pub fn paste_record_content(id: i64, app: AppHandle) -> CmdResult<()> {
    paste_record_logic(&app, id)
}
