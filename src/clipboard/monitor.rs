#[cfg(target_os = "windows")]
use clipboard_master::{CallbackResult, ClipboardHandler, Master};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 时间盾保护窗口（毫秒），用于过滤Windows平台可能出现的连发事件
const IGNORE_WINDOW_MS: u64 = 400;

// ==========================================
// 核心监控状态控制器 (State Controller)
// ==========================================

pub struct MonitorController {
    is_running: AtomicBool,
    session_id: AtomicU64,
    ignore_until_ms: AtomicU64,
    paste_in_progress: AtomicBool,
}

impl MonitorController {
    const fn new() -> Self {
        Self {
            is_running: AtomicBool::new(false),
            session_id: AtomicU64::new(0),
            ignore_until_ms: AtomicU64::new(0),
            paste_in_progress: AtomicBool::new(false),
        }
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    fn check_and_mark_running(&self) -> bool {
        self.is_running.swap(true, Ordering::SeqCst)
    }

    fn next_session_id(&self) -> u64 {
        self.session_id
            .fetch_add(1, Ordering::SeqCst)
            .saturating_add(1)
    }

    fn is_session_active(&self, session_id: u64) -> bool {
        self.is_running.load(Ordering::SeqCst)
            && self.session_id.load(Ordering::SeqCst) == session_id
    }

    pub fn mark_ignore_changes_for_a_while(&self) {
        self.ignore_until_ms
            .store(Self::now_ms() + IGNORE_WINDOW_MS, Ordering::SeqCst);
    }

    fn should_ignore(&self) -> bool {
        Self::now_ms() < self.ignore_until_ms.load(Ordering::SeqCst)
    }

    fn set_paste_in_progress(&self, in_progress: bool) {
        self.paste_in_progress.store(in_progress, Ordering::SeqCst);
    }

    fn is_paste_in_progress(&self) -> bool {
        self.paste_in_progress.load(Ordering::SeqCst)
    }
}

pub static CONTROLLER: MonitorController = MonitorController::new();

// ==========================================
// 辅助工具 & RAII 保护器
// ==========================================

pub struct PasteInProgressGuard;

impl PasteInProgressGuard {
    fn enter() -> Self {
        CONTROLLER.set_paste_in_progress(true);
        Self
    }
}

impl Drop for PasteInProgressGuard {
    fn drop(&mut self) {
        CONTROLLER.set_paste_in_progress(false);
    }
}

pub fn with_paste_in_progress<T, E>(f: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
    let _guard = PasteInProgressGuard::enter();
    f()
}

pub fn mark_ignore_next_change() {
    CONTROLLER.mark_ignore_changes_for_a_while();
}

// ==========================================
// 私有事件分发
// ==========================================

fn handle_clipboard_event(on_history_changed: &dyn Fn()) {
    if CONTROLLER.should_ignore() {
        return;
    }
    if CONTROLLER.is_paste_in_progress() {
        return;
    }

    match crate::clipboard::processor::process_clipboard_change_core() {
        Ok(true) => on_history_changed(),
        Ok(false) => {}
        Err(error) => eprintln!("剪贴板处理发生错误: {error}"),
    }
}

// ==========================================
// 平台特定的底层驱动引擎封装
// ==========================================

#[cfg(not(target_os = "windows"))]
fn run_polling_loop(session_id: u64, on_history_changed: Arc<dyn Fn() + Send + Sync>) {
    while CONTROLLER.is_session_active(session_id) {
        if CONTROLLER.is_paste_in_progress() {
            thread::sleep(Duration::from_millis(120));
            continue;
        }
        thread::sleep(Duration::from_millis(500));
        if !CONTROLLER.is_session_active(session_id) {
            break;
        }
        handle_clipboard_event(on_history_changed.as_ref());
    }
}

#[cfg(target_os = "windows")]
struct ClipboardEventHandler {
    session_id: u64,
    on_history_changed: Arc<dyn Fn() + Send + Sync>,
}

#[cfg(target_os = "windows")]
impl ClipboardHandler for ClipboardEventHandler {
    fn on_clipboard_change(&mut self) -> CallbackResult {
        if !CONTROLLER.is_session_active(self.session_id) {
            return CallbackResult::Stop;
        }
        handle_clipboard_event(self.on_history_changed.as_ref());
        CallbackResult::Next
    }

    fn on_clipboard_error(&mut self, error: std::io::Error) -> CallbackResult {
        eprintln!("系统级剪贴板监听错误: {}", error);
        if !CONTROLLER.is_session_active(self.session_id) {
            CallbackResult::Stop
        } else {
            CallbackResult::Next
        }
    }
}

#[cfg(target_os = "windows")]
fn spawn_event_driven_monitor(session_id: u64, on_history_changed: Arc<dyn Fn() + Send + Sync>) {
    thread::spawn(move || {
        let retry_min = 300;
        let retry_max = 3000;
        let mut retry_delay_ms = retry_min;

        while CONTROLLER.is_session_active(session_id) {
            let handler = ClipboardEventHandler {
                session_id,
                on_history_changed: on_history_changed.clone(),
            };

            match Master::new(handler) {
                Ok(mut master) => {
                    retry_delay_ms = retry_min;
                    let _ = master.run();
                }
                Err(e) => eprintln!("创建剪贴板 Master 钩子失败: {}", e),
            }

            if !CONTROLLER.is_session_active(session_id) {
                break;
            }

            thread::sleep(Duration::from_millis(retry_delay_ms));
            retry_delay_ms = (retry_delay_ms.saturating_mul(2)).min(retry_max);
        }
    });
}

// ==========================================
// 外部调用入口
// ==========================================

pub fn start_monitoring_with_callback(
    on_history_changed: impl Fn() + Send + Sync + 'static,
) -> Result<(), String> {
    if CONTROLLER.check_and_mark_running() {
        return Ok(());
    }

    let session_id = CONTROLLER.next_session_id();
    let on_history_changed: Arc<dyn Fn() + Send + Sync> = Arc::new(on_history_changed);

    if let Err(e) = crate::clipboard::processor::init_startup_signature() {
        eprintln!("初始化启动签名失败: {}", e);
    }

    #[cfg(target_os = "windows")]
    {
        spawn_event_driven_monitor(session_id, on_history_changed);
    }

    #[cfg(not(target_os = "windows"))]
    {
        thread::spawn(move || run_polling_loop(session_id, on_history_changed));
    }

    Ok(())
}
