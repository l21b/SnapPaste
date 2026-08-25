#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState, hotkey::HotKey};
#[cfg(target_os = "windows")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::{
    ComponentHandle, Image, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, Timer, TimerMode,
    VecModel, Weak,
};
use snappaste_lib::models::{ClipboardRecord, ContentType, Settings, Theme};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetCursorPos, GetForegroundWindow, GetWindowLongPtrW, GetWindowRect, HTCAPTION,
    HWND_NOTOPMOST, HWND_TOPMOST, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SendMessageW, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, WM_NCLBUTTONDOWN,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

slint::include_modules!();

const PAGE_SIZE: i32 = 200;
const THUMBNAIL_WIDTH: usize = 120;
const THUMBNAIL_HEIGHT: usize = 82;
const PROJECT_URL: &str = "https://github.com/l21b/SnapPaste";

struct ViewState {
    keyword: String,
    favorites_only: bool,
    preserve_target_focus: bool,
    shown_at: Instant,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            keyword: String::new(),
            favorites_only: false,
            preserve_target_focus: false,
            shown_at: Instant::now(),
        }
    }
}

struct HotkeyRuntime {
    manager: GlobalHotKeyManager,
    current: Option<HotKey>,
    active_id: Arc<AtomicU32>,
}

impl HotkeyRuntime {
    fn new(active_id: Arc<AtomicU32>) -> Result<Self, String> {
        Ok(Self {
            manager: GlobalHotKeyManager::new().map_err(|error| error.to_string())?,
            current: None,
            active_id,
        })
    }

    fn update(&mut self, value: &str) -> Result<(), String> {
        let value = value.trim();
        if value.is_empty() {
            return Err("主快捷键不能为空".to_string());
        }
        let next = value
            .parse::<HotKey>()
            .map_err(|error| format!("无效快捷键 {value}：{error}"))?;
        if self.current == Some(next) {
            return Ok(());
        }

        let previous = self.current;
        if let Some(previous) = previous {
            self.manager
                .unregister(previous)
                .map_err(|error| format!("无法注销旧快捷键：{error}"))?;
        }

        if let Err(error) = self.manager.register(next) {
            if let Some(previous) = previous
                && let Err(restore_error) = self.manager.register(previous)
            {
                eprintln!("[Hotkey] failed to restore previous hotkey: {restore_error}");
            }
            return Err(format!("注册快捷键 {value} 失败：{error}"));
        }

        self.current = Some(next);
        self.active_id.store(next.id(), Ordering::SeqCst);
        Ok(())
    }
}

fn lock_view_state(state: &Arc<Mutex<ViewState>>) -> MutexGuard<'_, ViewState> {
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn content_type_label(content_type: &ContentType) -> &'static str {
    match content_type {
        ContentType::Text => "文本",
        ContentType::Image => "图片",
        ContentType::Html => "HTML",
        ContentType::Link => "链接",
    }
}

fn display_content(record: &ClipboardRecord) -> String {
    let mut text = record.content.replace(['\r', '\n'], " ");
    const MAX_CHARS: usize = 180;
    if text.chars().count() > MAX_CHARS {
        text = text.chars().take(MAX_CHARS).collect::<String>() + "…";
    }
    text
}

fn image_thumbnail(record_id: i64) -> Result<Image, String> {
    let record = snappaste_lib::db::get_record_by_id(record_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "图片记录不存在".to_string())?;
    let png = record
        .image_data
        .ok_or_else(|| "图片记录缺少数据".to_string())?;
    let (width, height, rgba) = snappaste_lib::utils::image::decode_png_rgba(&png)?;
    if width == 0 || height == 0 {
        return Err("图片尺寸无效".to_string());
    }

    let ratio = (width as f64 / THUMBNAIL_WIDTH as f64)
        .max(height as f64 / THUMBNAIL_HEIGHT as f64)
        .max(1.0);
    let target_width = ((width as f64 / ratio).round() as usize).max(1);
    let target_height = ((height as f64 / ratio).round() as usize).max(1);
    let thumbnail = if target_width == width && target_height == height {
        rgba
    } else {
        snappaste_lib::utils::image::downscale_rgba_nearest(
            width,
            height,
            &rgba,
            target_width,
            target_height,
        )
    };
    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
        &thumbnail,
        target_width as u32,
        target_height as u32,
    );
    Ok(Image::from_rgba8(buffer))
}

fn to_ui_record(record: ClipboardRecord) -> ClipboardItemData {
    let has_image = record.content_type == ContentType::Image;
    let thumbnail = if has_image {
        image_thumbnail(record.id).unwrap_or_default()
    } else {
        Image::default()
    };
    ClipboardItemData {
        id: SharedString::from(record.id.to_string()),
        content: SharedString::from(display_content(&record)),
        type_label: SharedString::from(content_type_label(&record.content_type)),
        created_at: SharedString::from(record.created_at),
        favorite: record.is_favorite,
        pinned: record.is_pinned,
        has_image,
        thumbnail,
    }
}

fn query_records(state: &ViewState) -> Result<Vec<ClipboardRecord>, String> {
    let keyword = state.keyword.trim();
    match (state.favorites_only, keyword.is_empty()) {
        (false, true) => snappaste_lib::db::get_history(PAGE_SIZE, 0),
        (false, false) => snappaste_lib::db::search_history(keyword, PAGE_SIZE),
        (true, true) => snappaste_lib::db::get_favorites(PAGE_SIZE, 0),
        (true, false) => snappaste_lib::db::search_favorites(keyword, PAGE_SIZE),
    }
}

fn refresh_records(ui: &Weak<MainWindow>, state: &Arc<Mutex<ViewState>>) {
    let Some(ui) = ui.upgrade() else {
        return;
    };
    let (result, favorites_only) = {
        let state = lock_view_state(state);
        (query_records(&state), state.favorites_only)
    };

    match result {
        Ok(records) => {
            let model = VecModel::from(records.into_iter().map(to_ui_record).collect::<Vec<_>>());
            ui.set_records(ModelRc::new(model));
            ui.set_favorites_only(favorites_only);
            ui.set_status_text("".into());
        }
        Err(error) => ui.set_status_text(format!("加载失败：{error}").into()),
    }
}

fn parse_id(id: &str) -> Result<i64, String> {
    id.parse::<i64>()
        .map_err(|_| format!("无效的记录编号：{id}"))
}

fn copy_record_to_clipboard(id: i64) -> Result<(), String> {
    let record = snappaste_lib::db::get_record_by_id(id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "记录不存在或已被删除".to_string())?;

    match record.content_type {
        ContentType::Image => {
            let bytes = record
                .image_data
                .ok_or_else(|| "图片记录缺少数据".to_string())?;
            let (width, height, rgba) = snappaste_lib::utils::image::decode_png_rgba(&bytes)?;
            snappaste_lib::clipboard::monitor::mark_ignore_next_change();
            snappaste_lib::clipboard::access::write_image(width, height, rgba)
                .map_err(|error| error.to_string())
        }
        _ => {
            snappaste_lib::clipboard::monitor::mark_ignore_next_change();
            snappaste_lib::clipboard::access::write_text(&record.content)
                .map_err(|error| error.to_string())
        }
    }
}

fn paste_into_preserved_target(ui: &MainWindow) -> Result<(), String> {
    ui.hide().map_err(|error| error.to_string())?;
    snappaste_lib::clipboard::monitor::with_paste_in_progress(|| {
        snappaste_lib::keyboard::input::simulate_paste(20)
    })
}

fn theme_token(theme: &Theme) -> &'static str {
    match theme {
        Theme::System => "system",
        Theme::Light => "light",
        Theme::Dark => "dark",
    }
}

fn apply_theme(ui: &MainWindow, theme: &Theme) {
    let dark_mode = match theme {
        Theme::Dark => true,
        Theme::Light => false,
        Theme::System => snappaste_lib::slint_support::shell::system_uses_dark_theme(),
    };
    ui.global::<UiPalette>().set_dark_mode(dark_mode);
}

fn load_settings_into_ui(ui: &MainWindow) -> Result<Settings, String> {
    let settings = snappaste_lib::db::get_settings().map_err(|error| error.to_string())?;
    ui.set_settings_hotkey(settings.hotkey.clone().into());
    ui.set_settings_theme(theme_token(&settings.theme).into());
    ui.set_settings_keep_days(settings.keep_days.to_string().into());
    ui.set_settings_max_records(settings.max_records.to_string().into());
    ui.set_settings_auto_start(settings.auto_start);
    apply_theme(ui, &settings.theme);
    Ok(settings)
}

fn parse_settings_draft(
    previous: &Settings,
    hotkey: &str,
    theme: &str,
    keep_days: &str,
    max_records: &str,
    auto_start: bool,
) -> Result<Settings, String> {
    let hotkey = hotkey.trim();
    if hotkey.is_empty() {
        return Err("主快捷键不能为空".to_string());
    }
    hotkey
        .parse::<HotKey>()
        .map_err(|error| format!("无效快捷键 {hotkey}：{error}"))?;

    let keep_days = keep_days
        .trim()
        .parse::<i32>()
        .map_err(|_| "记录保留天数必须是整数".to_string())?;
    let max_records = max_records
        .trim()
        .parse::<i32>()
        .map_err(|_| "最大记录数必须是整数".to_string())?;
    if !(0..=3650).contains(&keep_days) {
        return Err("记录保留天数必须在 0 到 3650 之间".to_string());
    }
    if !(0..=100_000).contains(&max_records) {
        return Err("最大记录数必须在 0 到 100000 之间".to_string());
    }
    let theme = match theme {
        "system" => Theme::System,
        "light" => Theme::Light,
        "dark" => Theme::Dark,
        _ => return Err("未知的主题设置".to_string()),
    };

    let mut updated = previous.clone();
    updated.hotkey = hotkey.to_string();
    updated.theme = theme;
    updated.keep_days = keep_days;
    updated.max_records = max_records;
    updated.auto_start = auto_start;
    updated.ai_enabled = false;
    Ok(updated)
}

fn apply_settings_transaction(
    runtime: &mut HotkeyRuntime,
    previous: &Settings,
    updated: &Settings,
) -> Result<(), String> {
    runtime.update(&updated.hotkey)?;
    if let Err(error) = snappaste_lib::slint_support::autostart::sync(updated.auto_start) {
        let _ = runtime.update(&previous.hotkey);
        return Err(error);
    }
    if let Err(error) = snappaste_lib::db::save_settings(updated) {
        let _ = runtime.update(&previous.hotkey);
        if let Err(rollback_error) =
            snappaste_lib::slint_support::autostart::sync(previous.auto_start)
        {
            eprintln!("[Settings] failed to restore autostart state: {rollback_error}");
        }
        return Err(error.to_string());
    }
    Ok(())
}

fn bind_callbacks(
    ui: &MainWindow,
    state: Arc<Mutex<ViewState>>,
    hotkeys: Rc<RefCell<HotkeyRuntime>>,
) {
    let weak = ui.as_weak();
    ui.on_window_drag_requested(move || {
        if let Some(ui) = weak.upgrade()
            && let Err(error) = start_window_drag(&ui)
        {
            eprintln!("[Window] failed to begin drag: {error}");
        }
    });

    let weak = ui.as_weak();
    let search_state = state.clone();
    ui.on_search_changed(move |keyword| {
        lock_view_state(&search_state).keyword = keyword.to_string();
        refresh_records(&weak, &search_state);
    });

    let weak = ui.as_weak();
    let refresh_state = state.clone();
    ui.on_refresh_requested(move || refresh_records(&weak, &refresh_state));

    let weak = ui.as_weak();
    let favorites_state = state.clone();
    ui.on_favorites_toggled(move || {
        let mut state = lock_view_state(&favorites_state);
        state.favorites_only = !state.favorites_only;
        drop(state);
        refresh_records(&weak, &favorites_state);
    });

    let weak = ui.as_weak();
    let activate_state = state.clone();
    ui.on_record_activated(move |id| {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let result = parse_id(id.as_str()).and_then(copy_record_to_clipboard);
        let preserve_target_focus = lock_view_state(&activate_state).preserve_target_focus;
        match result {
            Ok(()) if preserve_target_focus => {
                if let Err(error) = paste_into_preserved_target(&ui) {
                    ui.set_status_text(format!("自动粘贴失败：{error}；内容已保留在剪贴板").into());
                    let _ = ui.show();
                }
            }
            Ok(()) => ui.set_status_text("已复制到剪贴板".into()),
            Err(error) => ui.set_status_text(format!("复制失败：{error}").into()),
        }
    });

    let weak = ui.as_weak();
    let delete_state = state.clone();
    ui.on_record_removed(move |id| {
        let result =
            parse_id(id.as_str()).and_then(|id| snappaste_lib::db::delete_item(id).map(|_| ()));
        if let Some(ui) = weak.upgrade()
            && let Err(error) = result
        {
            ui.set_status_text(format!("删除失败：{error}").into());
            return;
        }
        refresh_records(&weak, &delete_state);
    });

    let weak = ui.as_weak();
    let favorite_state = state.clone();
    ui.on_favorite_changed(move |id, favorite| {
        let result =
            parse_id(id.as_str()).and_then(|id| snappaste_lib::db::toggle_favorite(id, favorite));
        if let Some(ui) = weak.upgrade()
            && let Err(error) = result
        {
            ui.set_status_text(format!("更新收藏失败：{error}").into());
            return;
        }
        refresh_records(&weak, &favorite_state);
    });

    let weak = ui.as_weak();
    let pinned_state = state.clone();
    ui.on_pinned_changed(move |id, pinned| {
        let result =
            parse_id(id.as_str()).and_then(|id| snappaste_lib::db::toggle_pinned(id, pinned));
        if let Some(ui) = weak.upgrade()
            && let Err(error) = result
        {
            ui.set_status_text(format!("更新置顶失败：{error}").into());
            return;
        }
        refresh_records(&weak, &pinned_state);
    });

    let weak = ui.as_weak();
    let add_state = state.clone();
    ui.on_add_favorite_requested(move |content| {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        match snappaste_lib::db::add_custom_favorite_record_logic(content.to_string()) {
            Ok(_) => {
                ui.set_active_panel("main".into());
                ui.set_favorite_draft("".into());
                refresh_records(&ui.as_weak(), &add_state);
                ui.set_status_text("收藏已添加".into());
            }
            Err(error) => ui.set_status_text(format!("添加收藏失败：{error}").into()),
        }
    });

    let weak = ui.as_weak();
    let clear_state = state.clone();
    ui.on_clear_confirmed(move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let favorites_only = lock_view_state(&clear_state).favorites_only;
        let result = if favorites_only {
            snappaste_lib::db::clear_favorite_records()
        } else {
            snappaste_lib::db::clear_history_records()
        };
        match result {
            Ok(count) => {
                ui.set_active_panel("main".into());
                refresh_records(&ui.as_weak(), &clear_state);
                ui.set_status_text(format!("已清除 {count} 条记录").into());
            }
            Err(error) => ui.set_status_text(format!("清空失败：{error}").into()),
        }
    });

    let weak = ui.as_weak();
    let settings_state = state.clone();
    ui.on_settings_requested(move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        show_main_window(&ui, &settings_state);
        match load_settings_into_ui(&ui) {
            Ok(_) => ui.set_active_panel("settings".into()),
            Err(error) => ui.set_status_text(format!("读取设置失败：{error}").into()),
        }
    });

    let weak = ui.as_weak();
    let save_state = state.clone();
    let save_hotkeys = hotkeys.clone();
    ui.on_settings_save_requested(move |hotkey, theme, keep_days, max_records, auto_start| {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let result = (|| {
            let previous = snappaste_lib::db::get_settings().map_err(|error| error.to_string())?;
            let updated = parse_settings_draft(
                &previous,
                hotkey.as_str(),
                theme.as_str(),
                keep_days.as_str(),
                max_records.as_str(),
                auto_start,
            )?;
            apply_settings_transaction(&mut save_hotkeys.borrow_mut(), &previous, &updated)?;
            Ok::<Settings, String>(updated)
        })();

        match result {
            Ok(updated) => {
                apply_theme(&ui, &updated.theme);
                ui.set_active_panel("main".into());
                refresh_records(&ui.as_weak(), &save_state);
                ui.set_status_text("设置已保存并立即生效".into());
            }
            Err(error) => ui.set_status_text(format!("保存设置失败：{error}").into()),
        }
    });

    let weak = ui.as_weak();
    let import_state = state.clone();
    let import_hotkeys = hotkeys;
    ui.on_import_requested(move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .set_title("导入 SnapPaste 收藏")
            .add_filter("JSON 文件", &["json"])
            .pick_file()
        else {
            return;
        };
        let previous = match snappaste_lib::db::get_settings() {
            Ok(settings) => settings,
            Err(error) => {
                ui.set_status_text(format!("读取现有设置失败：{error}").into());
                return;
            }
        };
        match snappaste_lib::db::import_favorites_from_path_sync(&path.to_string_lossy()) {
            Ok((count, settings_imported)) => {
                if settings_imported {
                    let runtime_result = (|| {
                        let imported =
                            snappaste_lib::db::get_settings().map_err(|error| error.to_string())?;
                        let validated = parse_settings_draft(
                            &imported,
                            &imported.hotkey,
                            theme_token(&imported.theme),
                            &imported.keep_days.to_string(),
                            &imported.max_records.to_string(),
                            imported.auto_start,
                        )?;
                        apply_settings_transaction(
                            &mut import_hotkeys.borrow_mut(),
                            &previous,
                            &validated,
                        )?;
                        apply_theme(&ui, &validated.theme);
                        Ok::<(), String>(())
                    })();
                    if let Err(error) = runtime_result {
                        let _ = snappaste_lib::db::save_settings(&previous);
                        let _ = import_hotkeys.borrow_mut().update(&previous.hotkey);
                        let _ = snappaste_lib::slint_support::autostart::sync(previous.auto_start);
                        ui.set_status_text(
                            format!("已导入 {count} 条收藏，但设置应用失败：{error}").into(),
                        );
                        refresh_records(&ui.as_weak(), &import_state);
                        return;
                    }
                    let _ = load_settings_into_ui(&ui);
                }
                refresh_records(&ui.as_weak(), &import_state);
                ui.set_status_text(
                    format!(
                        "已导入 {count} 条收藏{}",
                        if settings_imported { "和设置" } else { "" }
                    )
                    .into(),
                );
            }
            Err(error) => ui.set_status_text(format!("导入失败：{error}").into()),
        }
    });

    let weak = ui.as_weak();
    ui.on_export_requested(move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let file_name = format!(
            "snappaste-favorites-{}.json",
            chrono::Local::now().format("%Y%m%d-%H%M%S")
        );
        let Some(path) = rfd::FileDialog::new()
            .set_title("导出 SnapPaste 收藏")
            .set_file_name(&file_name)
            .add_filter("JSON 文件", &["json"])
            .save_file()
        else {
            return;
        };
        match snappaste_lib::db::export_favorites_to_path_logic(path.to_string_lossy().to_string())
        {
            Ok(result) => ui.set_status_text(
                format!("已导出 {} 条收藏到 {}", result.count, result.path).into(),
            ),
            Err(error) => ui.set_status_text(format!("导出失败：{error}").into()),
        }
    });

    let weak = ui.as_weak();
    let about_state = state;
    ui.on_about_requested(move || {
        if let Some(ui) = weak.upgrade() {
            show_main_window(&ui, &about_state);
            ui.set_active_panel("about".into());
        }
    });

    let weak = ui.as_weak();
    ui.on_project_requested(move || {
        if let Some(ui) = weak.upgrade()
            && let Err(error) = snappaste_lib::slint_support::shell::open_url(PROJECT_URL)
        {
            ui.set_status_text(format!("打开项目地址失败：{error}").into());
        }
    });

    let weak = ui.as_weak();
    ui.on_escape_requested(move || {
        if let Some(ui) = weak.upgrade()
            && let Err(error) = ui.hide()
        {
            ui.set_status_text(format!("窗口隐藏失败：{error}").into());
        }
    });
}

fn start_clipboard_monitor(ui: &MainWindow, state: Arc<Mutex<ViewState>>) {
    let weak = ui.as_weak();
    if let Err(error) =
        snappaste_lib::clipboard::monitor::start_monitoring_with_callback(move || {
            let state = state.clone();
            if let Err(error) = weak.upgrade_in_event_loop(move |ui| {
                refresh_records(&ui.as_weak(), &state);
            }) {
                eprintln!("[Slint] failed to queue clipboard refresh: {error}");
            }
        })
    {
        ui.set_status_text(format!("剪贴板监听启动失败：{error}").into());
    }
}

#[cfg(target_os = "windows")]
fn main_window_hwnd(ui: &MainWindow) -> Result<HWND, String> {
    let handle = ui.window().window_handle();
    let handle = handle.window_handle().map_err(|error| error.to_string())?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get() as HWND),
        _ => Err("当前窗口不是 Win32 窗口".to_string()),
    }
}

#[cfg(target_os = "windows")]
fn configure_no_activate(ui: &MainWindow, enabled: bool) -> Result<(), String> {
    let hwnd = main_window_hwnd(ui)?;
    unsafe {
        let current_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let popup_flags = WS_EX_NOACTIVATE as isize;
        let next_style = if enabled {
            current_style | popup_flags | WS_EX_TOOLWINDOW as isize
        } else {
            current_style & !popup_flags | WS_EX_TOOLWINDOW as isize
        };
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next_style);

        let insert_after = if enabled {
            HWND_TOPMOST
        } else {
            HWND_NOTOPMOST
        };
        if SetWindowPos(
            hwnd,
            insert_after,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        ) == 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn start_window_drag(ui: &MainWindow) -> Result<(), String> {
    let hwnd = main_window_hwnd(ui)?;
    unsafe {
        ReleaseCapture();
        SendMessageW(hwnd, WM_NCLBUTTONDOWN, HTCAPTION as usize, 0);
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn start_window_drag(_ui: &MainWindow) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn position_near_cursor(ui: &MainWindow) -> Result<(), String> {
    let hwnd = main_window_hwnd(ui)?;
    unsafe {
        let mut cursor: POINT = std::mem::zeroed();
        if GetCursorPos(&mut cursor) == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }

        let monitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
        if monitor.is_null() {
            return Err("无法找到鼠标所在显示器".to_string());
        }

        let mut monitor_info: MONITORINFO = std::mem::zeroed();
        monitor_info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut monitor_info) == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }

        let mut window_rect: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut window_rect) == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }

        let width = window_rect.right - window_rect.left;
        let height = window_rect.bottom - window_rect.top;
        let work = monitor_info.rcWork;
        let margin = 8;
        let mut x = cursor.x + 12;
        let mut y = cursor.y + 16;

        if x + width > work.right - margin {
            x = (work.right - width - margin).max(work.left + margin);
        }
        if y + height > work.bottom - margin {
            y = (cursor.y - height - 12).max(work.top + margin);
        }
        x = x.max(work.left + margin);
        y = y.max(work.top + margin);

        if SetWindowPos(hwnd, HWND_TOPMOST, x, y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE) == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    Ok(())
}

fn show_main_window(ui: &MainWindow, state: &Arc<Mutex<ViewState>>) {
    {
        let mut state = lock_view_state(state);
        state.preserve_target_focus = false;
        state.shown_at = Instant::now();
    }
    ui.set_focus_preserving(false);
    ui.set_active_panel("main".into());
    #[cfg(target_os = "windows")]
    if let Err(error) = configure_no_activate(ui, false) {
        ui.set_status_text(format!("窗口模式恢复失败：{error}").into());
    }
    refresh_records(&ui.as_weak(), state);
    if let Err(error) = ui.show() {
        ui.set_status_text(format!("窗口显示失败：{error}").into());
        return;
    }
    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = main_window_hwnd(ui) {
        unsafe {
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

fn show_main_window_from_hotkey(ui: &MainWindow, state: &Arc<Mutex<ViewState>>) {
    {
        let mut state = lock_view_state(state);
        state.preserve_target_focus = true;
        state.shown_at = Instant::now();
    }
    ui.set_focus_preserving(true);
    ui.set_active_panel("main".into());
    refresh_records(&ui.as_weak(), state);

    #[cfg(target_os = "windows")]
    let no_activate_ready = match configure_no_activate(ui, true) {
        Ok(()) => true,
        Err(error) => {
            lock_view_state(state).preserve_target_focus = false;
            ui.set_focus_preserving(false);
            ui.set_status_text(format!("无焦点窗口模式失败：{error}").into());
            false
        }
    };

    #[cfg(not(target_os = "windows"))]
    {
        lock_view_state(state).preserve_target_focus = false;
        ui.set_focus_preserving(false);
    }

    if let Err(error) = ui.show() {
        ui.set_status_text(format!("窗口显示失败：{error}").into());
        return;
    }

    #[cfg(target_os = "windows")]
    if no_activate_ready && let Err(error) = position_near_cursor(ui) {
        ui.set_status_text(format!("窗口定位失败：{error}").into());
    }
}

fn bind_tray(tray: &AppTray, ui: &MainWindow, state: Arc<Mutex<ViewState>>) {
    let weak = ui.as_weak();
    let click_state = state.clone();
    tray.on_tray_clicked(move || {
        if let Some(ui) = weak.upgrade() {
            show_main_window(&ui, &click_state);
        }
    });

    let weak = ui.as_weak();
    let open_state = state.clone();
    tray.on_open_requested(move || {
        if let Some(ui) = weak.upgrade() {
            show_main_window(&ui, &open_state);
        }
    });

    let weak = ui.as_weak();
    let settings_state = state.clone();
    tray.on_settings_requested(move || {
        if let Some(ui) = weak.upgrade() {
            show_main_window(&ui, &settings_state);
            match load_settings_into_ui(&ui) {
                Ok(_) => ui.set_active_panel("settings".into()),
                Err(error) => ui.set_status_text(format!("读取设置失败：{error}").into()),
            }
        }
    });

    let weak = ui.as_weak();
    tray.on_about_requested(move || {
        if let Some(ui) = weak.upgrade() {
            show_main_window(&ui, &state);
            ui.set_active_panel("about".into());
        }
    });

    tray.on_quit_requested(|| {
        if let Err(error) = slint::quit_event_loop() {
            eprintln!("[Slint] failed to quit event loop: {error}");
        }
    });
}

fn install_hotkey_handler(
    ui: &MainWindow,
    state: Arc<Mutex<ViewState>>,
    active_id: Arc<AtomicU32>,
) {
    let weak = ui.as_weak();
    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        let expected_id = active_id.load(Ordering::SeqCst);
        if expected_id == 0 || event.id != expected_id || event.state != HotKeyState::Pressed {
            return;
        }

        let state = state.clone();
        if let Err(error) = weak.upgrade_in_event_loop(move |ui| {
            if ui.window().is_visible() {
                if let Err(error) = ui.hide() {
                    ui.set_status_text(format!("窗口隐藏失败：{error}").into());
                }
            } else {
                show_main_window_from_hotkey(&ui, &state);
            }
        }) {
            eprintln!("[Slint] failed to queue hotkey event: {error}");
        }
    }));
}

fn start_single_instance_timer(
    ui: &MainWindow,
    state: Arc<Mutex<ViewState>>,
    instance: Rc<snappaste_lib::slint_support::single_instance::SingleInstance>,
) -> Timer {
    let timer = Timer::default();
    let weak = ui.as_weak();
    timer.start(TimerMode::Repeated, Duration::from_millis(200), move || {
        if instance.take_show_request()
            && let Some(ui) = weak.upgrade()
        {
            show_main_window(&ui, &state);
        }
    });
    timer
}

fn start_auto_hide_timer(ui: &MainWindow, state: Arc<Mutex<ViewState>>) -> Timer {
    let timer = Timer::default();
    let weak = ui.as_weak();
    timer.start(TimerMode::Repeated, Duration::from_millis(150), move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        if !ui.window().is_visible() {
            return;
        }
        let panel = ui.get_active_panel();
        if panel.as_str() == "settings" || panel.as_str() == "about" {
            return;
        }
        let state = lock_view_state(&state);
        if state.preserve_target_focus || state.shown_at.elapsed() < Duration::from_millis(500) {
            return;
        }

        #[cfg(target_os = "windows")]
        if let Ok(hwnd) = main_window_hwnd(&ui) {
            let foreground = unsafe { GetForegroundWindow() };
            if !foreground.is_null() && foreground != hwnd {
                drop(state);
                if let Err(error) = ui.hide() {
                    ui.set_status_text(format!("窗口隐藏失败：{error}").into());
                }
            }
        }
    });
    timer
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(instance) = snappaste_lib::slint_support::single_instance::SingleInstance::acquire()?
    else {
        return Ok(());
    };
    let instance = Rc::new(instance);

    snappaste_lib::db::init_database()?;
    let settings = snappaste_lib::db::get_settings()?;
    if let Err(error) = snappaste_lib::slint_support::autostart::sync(settings.auto_start) {
        eprintln!("[Autostart] startup sync failed: {error}");
    }

    let ui = MainWindow::new()?;
    let tray = AppTray::new()?;
    ui.set_app_version(env!("CARGO_PKG_VERSION").into());
    apply_theme(&ui, &settings.theme);
    let _ = load_settings_into_ui(&ui);

    let state = Arc::new(Mutex::new(ViewState::default()));
    let active_hotkey_id = Arc::new(AtomicU32::new(0));
    let hotkeys = Rc::new(RefCell::new(HotkeyRuntime::new(active_hotkey_id.clone())?));
    if let Err(error) = hotkeys.borrow_mut().update(&settings.hotkey) {
        ui.set_status_text(format!("快捷键不可用：{error}").into());
    }
    install_hotkey_handler(&ui, state.clone(), active_hotkey_id);

    bind_callbacks(&ui, state.clone(), hotkeys);
    bind_tray(&tray, &ui, state.clone());
    refresh_records(&ui.as_weak(), &state);
    start_clipboard_monitor(&ui, state.clone());

    ui.window()
        .on_close_requested(|| slint::CloseRequestResponse::HideWindow);
    let _instance_timer = start_single_instance_timer(&ui, state.clone(), instance);
    let _auto_hide_timer = start_auto_hide_timer(&ui, state.clone());

    tray.show()?;
    let arguments = std::env::args().collect::<Vec<_>>();
    let show_on_start = arguments.iter().any(|argument| argument == "--show")
        || (cfg!(debug_assertions) && !arguments.iter().any(|argument| argument == "--autostart"));
    if show_on_start {
        show_main_window(&ui, &state);
    }
    slint::run_event_loop()?;
    GlobalHotKeyEvent::set_event_handler(None::<fn(GlobalHotKeyEvent)>);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_draft_updates_visible_fields_and_keeps_hidden_ai_values() {
        let previous = Settings {
            ai_enabled: true,
            ai_api_key: "keep-this-key".to_string(),
            ..Settings::default()
        };

        let updated = parse_settings_draft(&previous, "Ctrl+Shift+V", "dark", "0", "1000", true)
            .expect("valid settings should parse");

        assert_eq!(updated.hotkey, "Ctrl+Shift+V");
        assert_eq!(updated.theme, Theme::Dark);
        assert_eq!(updated.keep_days, 0);
        assert_eq!(updated.max_records, 1000);
        assert!(updated.auto_start);
        assert!(!updated.ai_enabled);
        assert_eq!(updated.ai_api_key, "keep-this-key");
    }

    #[test]
    fn settings_draft_rejects_invalid_limits() {
        let previous = Settings::default();

        assert!(
            parse_settings_draft(&previous, "Alt+Z", "system", "-1", "100", false)
                .unwrap_err()
                .contains("记录保留天数")
        );
        assert!(
            parse_settings_draft(&previous, "Alt+Z", "system", "1", "100001", false,)
                .unwrap_err()
                .contains("最大记录数")
        );
    }

    #[test]
    fn settings_draft_rejects_invalid_hotkey_and_theme() {
        let previous = Settings::default();

        assert!(parse_settings_draft(&previous, "", "system", "1", "100", false).is_err());
        assert!(parse_settings_draft(&previous, "Alt+Z", "sepia", "1", "100", false).is_err());
    }
}
