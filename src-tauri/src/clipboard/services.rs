use crate::db::queries::{get_settings, save_settings};
use crate::models::Settings;
use crate::ui::dialog::{DialogType, show_popup};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager}; // 引入 Duration

/// 保存应用设置
pub fn save_app_settings_logic(app: &AppHandle, settings: Settings) -> Result<(), String> {
    let mut updated = settings;
    updated.hotkey = updated.hotkey.trim().to_string();
    updated.keep_days = updated.keep_days.clamp(0, 3650);
    updated.max_records = updated.max_records.clamp(0, 100_000);
    updated.ai_enabled = crate::ai::FEATURE_ENABLED && updated.ai_enabled;

    if updated.hotkey.is_empty() {
        return Err("主快捷键不能为空".to_string());
    }
    if updated.ai_enabled
        && updated
            .hotkey
            .eq_ignore_ascii_case(updated.ai_hotkey.trim())
    {
        return Err("主快捷键和 AI 快捷键不能相同".to_string());
    }

    let previous = get_settings().map_err(|e| e.to_string())?;
    let manager = app.state::<crate::keyboard::hotkey::HotkeyManager>();

    if let Err(error) = apply_hotkey_settings(&manager, app, &updated) {
        restore_hotkey_settings(&manager, app, &previous);
        return Err(error);
    }

    if let Err(error) = crate::utils::autostart::set_enabled(app, updated.auto_start) {
        restore_hotkey_settings(&manager, app, &previous);
        return Err(error);
    }

    if let Err(error) = save_settings(&updated) {
        restore_hotkey_settings(&manager, app, &previous);
        if let Err(rollback_error) = crate::utils::autostart::set_enabled(app, previous.auto_start)
        {
            eprintln!("[Settings] failed to restore autostart state: {rollback_error}");
        }
        return Err(error.to_string());
    }

    Ok(())
}

fn apply_hotkey_settings(
    manager: &crate::keyboard::hotkey::HotkeyManager,
    app: &AppHandle,
    settings: &Settings,
) -> Result<(), String> {
    manager.register(app, "main", &settings.hotkey)?;
    if crate::ai::FEATURE_ENABLED && settings.ai_enabled {
        manager.register(app, "ai", settings.ai_hotkey.trim())?;
    } else {
        manager.unregister(app, "ai")?;
    }
    Ok(())
}

fn restore_hotkey_settings(
    manager: &crate::keyboard::hotkey::HotkeyManager,
    app: &AppHandle,
    settings: &Settings,
) {
    if let Err(error) = apply_hotkey_settings(manager, app, settings) {
        eprintln!("[Settings] failed to restore hotkeys: {error}");
    }
}

pub async fn process_ai_text_logic(
    client: &reqwest::Client,
    selected_text: &str,
) -> Result<String, String> {
    if selected_text.trim().is_empty() {
        return Err("selected text is empty".into());
    }

    let settings = get_settings().map_err(|e| e.to_string())?;
    crate::ai::client::process_text_with_settings(client, &settings, selected_text)
        .await
        .map_err(|e| e.to_string())
}

/// 处理按下"主快捷键"时的业务逻辑 (显示面板)
pub fn handle_main_shortcut(app: &AppHandle) {
    crate::ui::window_manager::capture_target_window();

    if !crate::ui::window_manager::is_frontend_ready() {
        crate::ui::window_manager::queue_show_near_cursor_on_ready();
        return;
    }

    if let Err(error) = crate::ui::window_manager::show_main_window_near_cursor(app) {
        eprintln!("[Window] failed to show near cursor: {error}");
    }
}

/// 处理按下"AI快捷键"时的业务逻辑 (调度大模型修正与写回)
pub fn handle_ai_shortcut(app: &AppHandle) {
    // 1. 确保在提取前拦截当前系统截图（业务前置要求）
    crate::ui::window_manager::capture_target_window();

    // 2. 调度键盘模块模拟复制并获取文本
    let _ = crate::keyboard::input::simulate_select_all();

    // 给操作系统足够时间来处理各种窗口焦点和选中文本的内部消息
    std::thread::sleep(Duration::from_millis(100));

    let selected_text = crate::keyboard::selection::get_selected_text();

    // 借助 processor.rs 作为唯一真理源，判断选中的是否是有效文本
    if selected_text.trim().is_empty() {
        show_popup(
            app,
            DialogType::Info,
            "提示",
            "未检测到文本，或选中的不是有效内容",
        );
        return;
    }

    // 3. 广播状态
    let _ = app.emit("AI_PROCESSING_START", ());
    let app_clone = app.clone();

    // 4. 将阻塞的网络请求踢给异步运行时
    tauri::async_runtime::spawn(async move {
        // 提取 Reqwest Client 的正确方式
        let client = app_clone.state::<reqwest::Client>().inner().clone();

        match process_ai_text_logic(&client, &selected_text).await {
            Ok(new_text) => {
                // 🛡️ 护盾 1：开启时间盾，无视接下来我们自己造成的剪贴板变动
                crate::clipboard::monitor::mark_ignore_next_change();

                // 调用原子操作回写
                if let Err(e) = crate::clipboard::access::write_text(&new_text) {
                    show_popup(
                        &app_clone,
                        DialogType::Error,
                        "写入剪贴板失败",
                        &e.to_string(),
                    );
                } else {
                    // 🚀 核心修复：这里不再引入 tokio 包袱，因为 50ms 级别在后台线程可以直接用 thread::sleep
                    std::thread::sleep(Duration::from_millis(50));

                    // 🛡️ 护盾 2：开启粘贴保护伞，防止底层 Monitor 误判
                    let paste_result = crate::clipboard::monitor::with_paste_in_progress(
                        || -> Result<(), String> {
                            crate::keyboard::input::simulate_paste(5).map_err(|e| e.to_string())
                        },
                    );

                    if let Err(e) = paste_result {
                        show_popup(&app_clone, DialogType::Error, "粘贴失败", &e.to_string());
                    }
                }
            }
            Err(err) => {
                show_popup(
                    &app_clone,
                    DialogType::Error,
                    "AI 处理失败",
                    &err.to_string(),
                );
            }
        }

        let _ = app_clone.emit("AI_PROCESSING_END", ());
    });
}

/// 核心业务：粘贴指定的历史记录内容
pub fn paste_record_logic(app: &AppHandle, id: i64) -> Result<(), String> {
    let record = crate::db::queries::get_record_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Record not found".to_string())?;

    // 1. 写入剪贴板（开启护盾防止 Monitor 重复录入）
    crate::clipboard::monitor::mark_ignore_next_change();

    match record.content_type {
        crate::models::ContentType::Image => {
            if let Some(bytes) = record.image_data {
                // 解码 PNG 为 RGBA
                let (w, h, rgba) =
                    crate::utils::image::decode_png_rgba(&bytes).map_err(|e| e.to_string())?;
                crate::clipboard::access::write_image(w, h, rgba).map_err(|e| e.to_string())?;
            } else {
                return Err("Image data missing".to_string());
            }
        }
        _ => {
            crate::clipboard::access::write_text(&record.content).map_err(|e| e.to_string())?;
        }
    }

    // 2. 隐藏面板并归还焦点
    crate::ui::window_manager::hide_main_window(app)?;

    // 3. 模拟粘贴（带一段小缓冲确保目标窗口已获得焦点）
    crate::clipboard::monitor::with_paste_in_progress(|| {
        crate::keyboard::input::simulate_paste(50)
    })
    .map_err(|e| e.to_string())?;

    Ok(())
}
