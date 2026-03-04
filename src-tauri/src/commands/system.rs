use tauri::AppHandle;
use crate::clipboard::services::process_ai_text_logic;
use super::CmdResult;

/// 暂停窗口自动隐藏
#[tauri::command]
pub fn suspend_auto_hide(ms: Option<u64>) {
    crate::suspend_main_window_auto_hide(ms.unwrap_or(4000));
}

/// 标记前端已就绪
#[tauri::command]
pub fn set_frontend_ready(app: AppHandle) {
    crate::mark_frontend_ready();
    if crate::take_pending_show_near_cursor() {
        let _ = crate::ui::window_manager::show_main_window_near_cursor(&app);
    }
}

/// 开始拖拽窗口
#[tauri::command]
pub fn start_window_drag(window: tauri::WebviewWindow) -> CmdResult<()> {
    window.start_dragging().map_err(|e| e.to_string())
}

/// 用默认浏览器打开链接
#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> CmdResult<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().open_url(&url, None::<String>).map_err(|e| e.to_string())
}

/// 粘贴 AI 处理结果
#[tauri::command]
pub fn paste_ai_result(text: String, _app: AppHandle) -> CmdResult<()> {
    if text.trim().is_empty() {
        return Err("text is empty".to_string());
    }
    use crate::clipboard::write_text;
    use crate::keyboard::keyboard::simulate_paste;
    write_text(&text).map_err(|e| e.to_string())?;
    simulate_paste(5).map_err(|e| e.to_string())?;
    Ok(())
}

/// 调用 AI 润色文本
#[tauri::command]
pub async fn process_ai_text(
    selected_text: String,
    client: tauri::State<'_, reqwest::Client>,
) -> CmdResult<String> {
    process_ai_text_logic(&client, &selected_text).await
}
