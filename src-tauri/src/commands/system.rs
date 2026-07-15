use super::CmdResult;
use tauri::AppHandle;

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
    let url = normalize_external_url(&url)?;
    app.opener()
        .open_url(&url, None::<String>)
        .map_err(|e| e.to_string())
}

fn normalize_external_url(value: &str) -> CmdResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("链接不能为空".to_string());
    }

    let normalized = if value.to_ascii_lowercase().starts_with("www.") {
        format!("https://{value}")
    } else {
        value.to_string()
    };
    let lower = normalized.to_ascii_lowercase();
    if !lower.starts_with("https://") && !lower.starts_with("http://") {
        return Err("只允许打开 HTTP 或 HTTPS 链接".to_string());
    }

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::normalize_external_url;

    #[test]
    fn external_url_accepts_http_and_adds_scheme_for_www() {
        assert_eq!(
            normalize_external_url("www.example.com").expect("normalize URL"),
            "https://www.example.com"
        );
        assert_eq!(
            normalize_external_url("https://example.com").expect("normalize URL"),
            "https://example.com"
        );
    }

    #[test]
    fn external_url_rejects_non_web_schemes() {
        assert!(normalize_external_url("file:///C:/secret.txt").is_err());
        assert!(normalize_external_url("javascript:alert(1)").is_err());
    }
}
