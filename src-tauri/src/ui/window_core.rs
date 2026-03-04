use tauri::{Monitor, PhysicalPosition, Window};

#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
};

// =============================================================================
// 1. 操作系统焦点操控原语 (OS Focus Management)
// =============================================================================

/// 捕获当前活动窗口的句柄 (HWND)
///
/// 在弹出主界面前调用，记录用户当前正在使用的窗口，以便后续隐藏时能够准确归还焦点。
#[cfg(target_os = "windows")]
pub fn capture_active_window_hwnd() -> isize {
    unsafe { GetForegroundWindow() as isize }
}

/// 非 Windows 平台占位实现
#[cfg(not(target_os = "windows"))]
pub fn capture_active_window_hwnd() -> isize {
    0
}

/// 强行将系统焦点归还给指定的窗口句柄 (HWND)
#[cfg(target_os = "windows")]
pub fn force_restore_focus(hwnd: isize) -> bool {
    unsafe {
        if hwnd == 0 {
            return false;
        }
        // 如果目标已经是前台，无需后续操作
        if GetForegroundWindow() as isize == hwnd {
            return true;
        }

        let target_hwnd = hwnd as *mut std::ffi::c_void;
        // 获取目标窗口的所属线程
        let target_thread = GetWindowThreadProcessId(target_hwnd, std::ptr::null_mut());
        if target_thread == 0 {
            return false;
        }

        let current_thread = GetCurrentThreadId();
        // 如果目标和当前处于同一线程，直接设置即可
        if current_thread == target_thread {
            return SetForegroundWindow(target_hwnd) != 0;
        }

        // 核心步骤：线程输入附加（挂靠）
        let _ = AttachThreadInput(current_thread, target_thread, 1);
        let focused = SetForegroundWindow(target_hwnd) != 0;
        // 断开挂靠
        let _ = AttachThreadInput(current_thread, target_thread, 0);
        focused
    }
}

// =============================================================================
// 2. 窗口几何与定位纯函数 (Geometry & Layout Pure Functions)
// =============================================================================

/// 获取窗口当前所在的显示器，如果获取失败则回退到主显示器
pub fn get_window_monitor(window: &Window) -> Option<Monitor> {
    window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten())
}

/// 计算指定显示器的工作区域边界（扣除任务栏，并留出指定的边缘边距）
pub fn get_monitor_work_area_bounds(monitor: &Monitor, margin_px: f64) -> (f64, f64, f64, f64) {
    let area = monitor.work_area();
    let left = area.position.x as f64 + margin_px;
    let top = area.position.y as f64 + margin_px;
    let right = area.position.x as f64 + area.size.width as f64 - margin_px;
    let bottom = area.position.y as f64 + area.size.height as f64 - margin_px;
    (left, top, right, bottom)
}

/// [纯算法] 根据光标位置计算窗口建议显示的坐标，并处理“屏幕边缘溢出”保护
pub fn calc_near_cursor_position(
    cursor_x: f64,
    cursor_y: f64,
    win_w: u32,
    win_h: u32,
    monitor_work_area: Option<(f64, f64, f64, f64)>,
) -> (i32, i32) {
    let mut x = cursor_x + 12.0;
    let mut y = cursor_y + 16.0;

    if let Some((left, top, right, bottom)) = monitor_work_area {
        // 如果底部溢出，反转到光标上方显示
        if y + win_h as f64 > bottom {
            y = cursor_y - win_h as f64 - 12.0;
        }
        // 强制约束在安全范围内
        x = x.clamp(left, (right - win_w as f64).max(left));
        y = y.clamp(top, (bottom - win_h as f64).max(top));
    } else {
        x = x.max(0.0);
        y = y.max(0.0);
    }
    (x.round() as i32, y.round() as i32)
}

/// 判断鼠标当前是否在窗口附近（包括窗口内以及向外延伸的 margin 区域）
pub fn is_cursor_near_window(window: &Window, margin_px: f64) -> Option<bool> {
    let cursor = window.cursor_position().ok()?;
    let position = window.outer_position().ok()?;
    let size = window.outer_size().ok()?;

    let left = position.x as f64 - margin_px;
    let top = position.y as f64 - margin_px;
    let right = position.x as f64 + size.width as f64 + margin_px;
    let bottom = position.y as f64 + size.height as f64 + margin_px;

    Some(cursor.x >= left && cursor.x <= right && cursor.y >= top && cursor.y <= bottom)
}

/// 强制将窗口位置限制在当前显示器的工作区域内
pub fn clamp_window_to_work_area(window: &Window) {
    let Ok(position) = window.outer_position() else {
        return;
    };
    let Ok(size) = window.outer_size() else {
        return;
    };
    let Some(monitor) = get_window_monitor(window) else {
        return;
    };

    let area = monitor.work_area();
    let min_x = area.position.x;
    let min_y = area.position.y;
    let max_x = (area.position.x + area.size.width as i32 - size.width as i32).max(min_x);
    let max_y = (area.position.y + area.size.height as i32 - size.height as i32).max(min_y);

    let clamped_x = position.x.clamp(min_x, max_x);
    let clamped_y = position.y.clamp(min_y, max_y);

    if clamped_x != position.x || clamped_y != position.y {
        let _ = window.set_position(PhysicalPosition::new(clamped_x, clamped_y));
    }
}
