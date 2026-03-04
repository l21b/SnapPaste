mod ai;
mod clipboard;
mod commands;
mod db;
mod models;

mod keyboard;
mod ui;
mod utils;

use tauri::Manager;
use tauri_plugin_global_shortcut::ShortcutState;

pub use ui::window_manager::{
    is_frontend_ready, mark_frontend_ready, mark_main_window_shown,
    queue_show_near_cursor_on_ready, suspend_main_window_auto_hide, take_pending_show_near_cursor,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    ui::window_manager::reset_run_state();

    tauri::Builder::default()
        // 1. 注入依赖
        .manage(keyboard::hotkey::HotkeyManager::new())
        // 2. 加载插件
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        // 3. 处理单实例运行
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = crate::ui::window_manager::show_main_window(app);
        }))
        // 4. 注册快捷键路由
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        let manager = app.state::<keyboard::hotkey::HotkeyManager>();
                        match manager.route(shortcut).as_deref() {
                            Some("main") => clipboard::services::handle_main_shortcut(app),
                            Some("ai") => clipboard::services::handle_ai_shortcut(app),
                            _ => {}
                        }
                    }
                })
                .build(),
        )
        // 4. 注册前端命令
        .invoke_handler(tauri::generate_handler![
            // 记录与数据类
            commands::get_history_records,
            commands::search_records,
            commands::get_favorite_records,
            commands::search_favorite_records,
            commands::add_custom_favorite_record,
            commands::delete_clipboard_record,
            commands::clear_history_only,
            commands::clear_favorite_items,
            commands::set_record_favorite_state,
            commands::set_record_pinned_state,
            commands::export_favorites_to_path,
            commands::import_favorites_from_path,
            // 设置类
            commands::get_app_settings,
            commands::save_app_settings,
            // 系统与外设类
            commands::suspend_auto_hide,
            commands::set_frontend_ready,
            commands::start_window_drag,
            commands::open_url,
            commands::paste_ai_result,
            commands::process_ai_text,
            commands::paste_record_content,
        ])
        // 5. 应用生命周期：初始化
        .setup(|app| {
            let handle = app.handle();

            // 1. 注入 AI 客户端
            let client =
                crate::ai::ai::create_ai_client().map_err(Box::<dyn std::error::Error>::from)?;
            app.manage(client);

            // 2. 初始化底层数据
            let _ = db::init_database();

            // 3. 主窗口
            crate::ui::window_manager::configure_main_window(handle);

            // 4. 初始化各项桌面端系统服务
            let _ = ui::tray::create_tray(handle);
            let _ = utils::autostart::sync_from_settings(handle);

            // 5. 注册底层快捷键
            let manager = app.state::<keyboard::hotkey::HotkeyManager>();
            if let Ok(settings) = db::queries::get_settings() {
                if let Err(e) = manager.register(handle, "main", &settings.hotkey) {
                    eprintln!("[Hotkey] failed to register main hotkey on startup: {}", e);
                }
                if settings.ai_enabled {
                    if let Err(e) = manager.register(handle, "ai", &settings.ai_hotkey) {
                        eprintln!("[Hotkey] failed to register AI hotkey on startup: {}", e);
                    }
                }
            }

            // 6. 开启后台守护线程
            let monitor_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                let _ = clipboard::monitor::start_monitoring(monitor_handle).await;
            });

            Ok(())
        })
        // 6. 窗口事件托管给 window.rs 处理
        .on_window_event(|window, event| {
            crate::ui::window_manager::handle_window_event(window, event);
        })
        // 7. 启动引擎
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
