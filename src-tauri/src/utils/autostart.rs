use tauri::{AppHandle, Runtime};
use tauri_plugin_autostart::ManagerExt;

/// 配置程序是否开启开机自启
///
/// 依赖 `tauri_plugin_autostart` 插件，将成功或失败的错误信息转化为标准的 `String`。
pub fn set_enabled<R: Runtime>(app: &AppHandle<R>, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    let currently_enabled = manager.is_enabled().unwrap_or(false);

    // 幂等性：状态一致直接返回
    if enabled == currently_enabled {
        return Ok(());
    }

    if enabled {
        manager
            .enable()
            .map_err(|e| format!("无法启用开机自启: {}", e))?;
    } else if let Err(e) = manager.disable() {
        // 关闭失败通常不会导致致命级错误，仅做打印警告
        eprintln!("[WARN] 禁用开机自启失败: {}", e);
    }
    Ok(())
}

/// 从数据库的配置中同步自启状态到系统中
pub fn sync_from_settings<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let settings = crate::db::queries::get_settings().map_err(|e| e.to_string())?;
    set_enabled(app, settings.auto_start)
}
