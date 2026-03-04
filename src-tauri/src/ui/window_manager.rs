use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
#[cfg(target_os = "windows")]
use std::sync::atomic::AtomicIsize;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Window, WindowEvent};

use crate::ui::window_core;

// =============================================================================
// 业务常量配置 (Business Constants)
// =============================================================================
/// 主窗口在 Tauri 中的唯一标识符
const MAIN_WINDOW_LABEL: &str = "main";
/// 几何变形焦点保护期：改变窗口位置/大小时系统可能会瞬间抽走焦点，这期间屏蔽自动隐藏
const GEOMETRY_FOCUS_GUARD_MS: u64 = 120;
/// 展现保护期：防止刚显示窗口时的系统环境抖动被误判为用户的“拖拽位置”事件
const SHOW_GEOMETRY_SUPPRESS_MS: u64 = 260;
/// 鼠标徘徊吸附边距：鼠标在窗口四周此范围内（像素）徘徊时，不会被判定为远离，免于失焦隐藏
const CURSOR_NEAR_WINDOW_MARGIN_PX: f64 = 8.0;
/// 窗口安全边距：防止通过计算放置窗口时被操作系统的边缘彻底遮盖或引发视差 Bug
const WINDOW_EDGE_MARGIN_PX: f64 = 6.0;

/// 失去焦点后的短时再次检测隐藏的时间（应对焦点瞬间被系统的剪贴板/右键菜单抢夺的情况）
const HIDE_RECHECK_SHORT_MS: u64 = 120;
/// 失去焦点后的长时再次检测隐藏的时间，构成二次判定屏障
const HIDE_RECHECK_LONG_MS: u64 = 240;
/// 因变形引起的补偿性延迟，配合 GEOMETRY_FOCUS_GUARD_MS 使用
const HIDE_RECHECK_GEOMETRY_EXTRA_MS: u64 = 96;
/// 焦点强制归还受害者后，停顿的心跳时间，防止此时我们立刻切回后台导致系统死锁或焦点全丢
const FOCUS_RESTORE_SETTLE_MS: u64 = 20;


// =============================================================================
// 业务状态机 (AppState)
// 这是一个由全局 Atomic 变量构成的免锁状态机，极其优雅地控制各种复杂的防抖状态。
// =============================================================================

/// 记录最后一次发生尺寸/位置变化的时间戳
static LAST_GEOMETRY_EVENT_MS: AtomicU64 = AtomicU64::new(0);
/// 记录主窗口最后一次被唤醒并展示的时间戳
static LAST_MAIN_WINDOW_SHOW_MS: AtomicU64 = AtomicU64::new(0);
/// 阻止窗口自动隐藏的"无敌防护罩"到期时间（例如此时用户点开了某个系统菜单，绝对不能隐藏主窗）
static AUTO_HIDE_SUSPEND_UNTIL_MS: AtomicU64 = AtomicU64::new(0);

/// 前端 Web 页面是否已经完全挂载并渲染完毕（防止在白屏时强制呼出）
static FRONTEND_READY: AtomicBool = AtomicBool::new(false);
/// 指令积压器：若前端未准备好就接到呼出指令，会挂起在此处，待就绪后立即触发
static PENDING_SHOW_NEAR_CURSOR: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "windows")]
/// （Windows专用）记录被我们强行抢走焦点的“倒霉受害者”窗口句柄，供退出时“完璧归赵”
static TARGET_HWND: AtomicIsize = AtomicIsize::new(0);

pub struct AppState;

impl AppState {    
    fn now_ms() -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
    }

    pub fn reset_run_state() {
        FRONTEND_READY.store(false, Ordering::SeqCst);
        PENDING_SHOW_NEAR_CURSOR.store(false, Ordering::SeqCst);
    }
    
    pub fn mark_main_window_shown() { LAST_MAIN_WINDOW_SHOW_MS.store(Self::now_ms(), Ordering::SeqCst); }
    fn last_main_window_show_ms() -> u64 { LAST_MAIN_WINDOW_SHOW_MS.load(Ordering::SeqCst) }
    
    fn mark_geometry_event() { LAST_GEOMETRY_EVENT_MS.store(Self::now_ms(), Ordering::SeqCst); }
    fn last_geometry_event_ms() -> u64 { LAST_GEOMETRY_EVENT_MS.load(Ordering::SeqCst) }
    
    fn auto_hide_is_suspended() -> bool { Self::now_ms() < AUTO_HIDE_SUSPEND_UNTIL_MS.load(Ordering::SeqCst) }
    
    /// 强制开启防护罩，在指定毫秒数内主窗口绝对不因失去系统焦点而隐藏
    pub fn suspend_main_window_auto_hide(ms: u64) {
        let duration_ms = ms.clamp(200, 15_000);
        AUTO_HIDE_SUSPEND_UNTIL_MS.store(Self::now_ms().saturating_add(duration_ms), Ordering::SeqCst);
    }
    
    pub fn mark_frontend_ready() { FRONTEND_READY.store(true, Ordering::SeqCst); }
    pub fn is_frontend_ready() -> bool { FRONTEND_READY.load(Ordering::SeqCst) }
    
    pub fn queue_show_near_cursor_on_ready() { PENDING_SHOW_NEAR_CURSOR.store(true, Ordering::SeqCst); }
    pub fn take_pending_show_near_cursor() -> bool { PENDING_SHOW_NEAR_CURSOR.swap(false, Ordering::SeqCst) }

    /// 截取当前处于前台的窗口（受害者）以便后续归还，必须在弹出我们的主窗口前一刻调用
    pub fn capture_target_window() {
        #[cfg(target_os = "windows")]
        TARGET_HWND.store(window_core::capture_active_window_hwnd(), Ordering::SeqCst);
    }
    
    #[cfg(target_os = "windows")]
    fn take_target_window() -> isize { TARGET_HWND.swap(0, Ordering::SeqCst) }
}

// =============================================================================
// 业务辅助方法 (Helpers)
// =============================================================================

fn get_main_window(app: &AppHandle) -> Option<tauri::WebviewWindow> {
    app.get_webview_window(MAIN_WINDOW_LABEL)
}

fn load_saved_main_window_size(window: &tauri::WebviewWindow) {
    if let Ok(Some((w, h))) = crate::db::queries::get_window_state(window.label()) {
        let scale_factor = window.scale_factor().unwrap_or(1.0);
        let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize::new(
            (w as f64 * scale_factor) as u32,
            (h as f64 * scale_factor) as u32,
        )));
    }
}

fn persist_main_window_size(window: &tauri::WebviewWindow) {
    if let Ok(size) = window.inner_size() {
        let scale_factor = window.scale_factor().unwrap_or(1.0);
        let w = (size.width as f64 / scale_factor) as u32;
        let h = (size.height as f64 / scale_factor) as u32;
        let _ = crate::db::queries::save_window_state(window.label(), w, h);
    }
}

/// 派遣“延时刺客”做二次确认：在一段时间后复查窗口如果依旧失焦且游离，则无情隐藏它。
fn schedule_hide_recheck(window: Window, delay_ms: u64) {
    tauri::async_runtime::spawn(async move {
        // 让当前任务交还调度器，到时间后再醒来
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        
        // 醒来后如果发现当前开启了无敌防护罩，直接撤退赦免
        if AppState::auto_hide_is_suspended() { return; }
        
        let unfocused = window.is_focused().map(|f| !f).unwrap_or(true);
        let near = window_core::is_cursor_near_window(&window, CURSOR_NEAR_WINDOW_MARGIN_PX);
        
        // 双杀条件达成：仍然没有焦点 AND 鼠标真的远离了窗口范围
        if unfocused && near == Some(false) {
            let _ = window.hide();
        }
    });
}

// =============================================================================
// 暴露的业务指令 (供主程序调用) (Exposed APIs)
// =============================================================================

/// 对刚拉起的主窗口进行底层配置（如剔除任务栏图标、限定最小尺寸）
pub fn configure_main_window(app: &AppHandle) {
    if let Some(main_window) = get_main_window(app) {
        let _ = main_window.set_skip_taskbar(true);
        let scale_factor = main_window.scale_factor().unwrap_or(1.0);

        let min_size = tauri::Size::Physical(tauri::PhysicalSize::new(
            (280.0 * scale_factor) as u32,
            (430.0 * scale_factor) as u32,
        ));
        let _ = main_window.set_min_size(Some(min_size));

        load_saved_main_window_size(&main_window);
    }
}

/// 全局级事件中枢管家：统一处理 Tauri 底层冒出的各类杂音事件
pub fn handle_window_event(window: &Window, event: &WindowEvent) {
    match event {
        // [事件拦截] 拦截常规的“X”关闭事件，将其转化为后台隐藏（保持应用常驻）
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            let _ = window.hide();
        }
        
        // [窗口游走监控] 当窗口因为拖拽而变形移位时
        WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
            if window.label() == MAIN_WINDOW_LABEL {
                let elapsed = AppState::now_ms().saturating_sub(AppState::last_main_window_show_ms());
                // 如果距离它刚显现还很短，说明这是系统级的初始化强制排版，忽略它
                if elapsed >= SHOW_GEOMETRY_SUPPRESS_MS {
                    AppState::mark_geometry_event();
                }
                // 每一次微小移动，都使用引力算法将其牢牢锁在当前工作区(屏幕)内，绝不溢出
                window_core::clamp_window_to_work_area(window); 
            }
        }
        
        // [失焦审判] 系统收回了我们的输入焦点。此时它是最危险和最需提防的时序。
        WindowEvent::Focused(false) => {
            if window.label() != MAIN_WINDOW_LABEL || AppState::auto_hide_is_suspended() {
                return;
            }

            // 防雷策略 A：鼠标探雷。
            // 往往用户并没有走开，只是点了一下窗口旁边的其他应用或系统栏，鼠标还在我们的地盘上。
            // 此时绝不是真正的意愿离开，所以释放两个“延时刺客”代替立即处决。
            match window_core::is_cursor_near_window(window, CURSOR_NEAR_WINDOW_MARGIN_PX) {
                Some(true) | None => {
                    schedule_hide_recheck(window.clone(), HIDE_RECHECK_SHORT_MS);
                    schedule_hide_recheck(window.clone(), HIDE_RECHECK_LONG_MS);
                    return;
                }
                Some(false) => {}
            }

            // 防雷策略 B：几何动荡。
            // 如果刚刚还触发过大小调节/位置变动，意味着底层正在经过剧烈渲染重回排，极易瞬时失焦。
            // 按照变动前的时间差给出余量赦免器。
            let elapsed = AppState::now_ms().saturating_sub(AppState::last_geometry_event_ms());
            if elapsed < GEOMETRY_FOCUS_GUARD_MS {
                let delay = GEOMETRY_FOCUS_GUARD_MS.saturating_sub(elapsed) + 16;
                schedule_hide_recheck(window.clone(), delay);
                schedule_hide_recheck(window.clone(), delay + HIDE_RECHECK_GEOMETRY_EXTRA_MS);
                return;
            }

            // [终极制裁] 所有豁免条件都没满足：那就是用户真的走开了，果断将窗口击隐于无形。
            let _ = window.hide();
        }
        _ => {}
    }
}

/// 将主面板在屏幕中央召唤显现
pub fn show_main_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = get_main_window(app) {
        let _ = window.center();
        AppState::mark_main_window_shown();
        let _ = app.emit("main-window-opened", ());
        window.show()?;
        window.set_focus()?;
    }
    Ok(())
}

/// [高级呼出] “随叫随到”：将主面板召唤并停靠在鼠标所在的位置
pub fn show_main_window_near_cursor(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = get_main_window(app) {
        let cursor = window.cursor_position().ok();
        let win_size = window.outer_size().ok();

        if let (Some(cursor), Some(size)) = (cursor, win_size) {
            // 借用底层的定位黑魔法，结合当前显示器信息强行算出一个绝不溢出的绝美位置
            let wv: &tauri::Webview = window.as_ref();
            let native_win = wv.window();
            let monitor = window_core::get_window_monitor(&native_win);
            let work_area = monitor.map(|m| window_core::get_monitor_work_area_bounds(&m, WINDOW_EDGE_MARGIN_PX));
            
            let (x, y) = window_core::calc_near_cursor_position(cursor.x, cursor.y, size.width, size.height, work_area);
            let _ = window.set_position(PhysicalPosition::new(x, y));
        }

        AppState::mark_main_window_shown();
        let _ = app.emit("main-window-opened", ());
        window.show()?;
        window.set_focus()?;
    }
    Ok(())
}

/// 隐藏主窗口 (包含受害者的焦点归还仪式)
pub fn hide_main_window(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = get_main_window(app) {
        window.hide().map_err(|e| e.to_string())?;

        // 当我们撤退时，如果有某个无辜的倒霉蛋曾经被我们抢走了焦点，在这里归还给它
        #[cfg(target_os = "windows")]
        {
            let target_hwnd = AppState::take_target_window();
            if target_hwnd != 0 {
                let _ = window_core::force_restore_focus(target_hwnd); 
                // 必须稍等这口气喘匀，不然后续如果在桌面端有瞬间的复制操作，系统分发队列会直接宕机
                std::thread::sleep(std::time::Duration::from_millis(FOCUS_RESTORE_SETTLE_MS));
            }
        }
    }
    Ok(())
}

/// 保存当前的形态然后光荣结束主进程生命周期
pub fn save_size_and_exit(app: &AppHandle) {
    if let Some(window) = get_main_window(app) {
        persist_main_window_size(&window);
    }
    app.exit(0);
}

// =============================================================================
// 模块级自由函数：将 AppState 方法提升为自由函数，供外部直接 use / re-export
// =============================================================================

pub fn reset_run_state()                        { AppState::reset_run_state() }
pub fn mark_main_window_shown()                 { AppState::mark_main_window_shown() }
pub fn mark_frontend_ready()                    { AppState::mark_frontend_ready() }
pub fn is_frontend_ready() -> bool              { AppState::is_frontend_ready() }
pub fn queue_show_near_cursor_on_ready()        { AppState::queue_show_near_cursor_on_ready() }
pub fn take_pending_show_near_cursor() -> bool  { AppState::take_pending_show_near_cursor() }
pub fn suspend_main_window_auto_hide(ms: u64)   { AppState::suspend_main_window_auto_hide(ms) }
pub fn capture_target_window()                  { AppState::capture_target_window() }