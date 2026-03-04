use super::CmdResult;
use crate::clipboard::services::save_app_settings_logic;
use crate::db::get_settings;
use crate::models::Settings;
use tauri::AppHandle;

/// 获取应用设置
#[tauri::command]
pub fn get_app_settings() -> CmdResult<Settings> {
    get_settings().map_err(|e| e.to_string())
}

/// 保存应用设置
#[tauri::command]
pub fn save_app_settings(app: AppHandle, settings: Settings) -> CmdResult<()> {
    save_app_settings_logic(&app, settings)
}
