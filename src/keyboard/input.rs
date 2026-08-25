use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;

const KEY_STEP_MS: u64 = 2;

// ==========================================
// 辅助与初始化 (Initialization & Helpers)
// ==========================================

/// 创建一个 Enigo 实例 (适配 enigo 0.6.1 的 Settings API)
fn create_enigo() -> Result<Enigo, String> {
    Enigo::new(&Settings::default()).map_err(|e| format!("无法初始化键盘系统: {}", e))
}

fn send_key(enigo: &mut Enigo, key: Key, direction: Direction, action: &str) -> Result<(), String> {
    enigo
        .key(key, direction)
        .map_err(|error| format!("{action}失败: {error}"))
}

fn clear_modifiers_with(enigo: &mut Enigo) -> Result<(), String> {
    let mut first_error = None;
    for (key, name) in [
        (Key::Control, "释放 Ctrl"),
        (Key::Alt, "释放 Alt"),
        (Key::Shift, "释放 Shift"),
        (Key::Meta, "释放 Meta"),
    ] {
        if let Err(error) = send_key(enigo, key, Direction::Release, name)
            && first_error.is_none()
        {
            first_error = Some(error);
        }
    }

    thread::sleep(Duration::from_millis(1));
    first_error.map_or(Ok(()), Err)
}

fn paste_key() -> Key {
    #[cfg(target_os = "windows")]
    {
        // Windows VK_V。直接使用虚拟键，避免输入法或键盘布局把字符事件解释为文本。
        Key::Other(0x56)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Key::Unicode('v')
    }
}

// ==========================================
// 核心模拟操作 (Simulation API)
// ==========================================

/// 模拟按下系统粘贴快捷键 (Windows/Linux: Ctrl+V, macOS: Cmd+V)
pub fn simulate_paste(delay_ms: u64) -> Result<(), String> {
    if delay_ms > 0 {
        thread::sleep(Duration::from_millis(delay_ms));
    }

    let mut enigo = create_enigo()?;
    clear_modifiers_with(&mut enigo)?;

    #[cfg(target_os = "macos")]
    {
        send_key(&mut enigo, Key::Meta, Direction::Press, "按下 Meta")?;
        thread::sleep(Duration::from_millis(KEY_STEP_MS));
        let paste_result = send_key(&mut enigo, paste_key(), Direction::Click, "发送粘贴键");
        thread::sleep(Duration::from_millis(KEY_STEP_MS));
        let release_result = send_key(&mut enigo, Key::Meta, Direction::Release, "释放 Meta");
        paste_result?;
        release_result?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        send_key(&mut enigo, Key::Control, Direction::Press, "按下 Ctrl")?;
        thread::sleep(Duration::from_millis(KEY_STEP_MS));
        let paste_result = send_key(&mut enigo, paste_key(), Direction::Click, "发送粘贴键");
        thread::sleep(Duration::from_millis(KEY_STEP_MS));
        let release_result = send_key(&mut enigo, Key::Control, Direction::Release, "释放 Ctrl");
        paste_result?;
        release_result?;
    }

    Ok(())
}

/// 模拟按下 Windows 复制快捷键 Ctrl+C
#[cfg(target_os = "windows")]
pub fn simulate_copy(delay_ms: u64) -> Result<(), String> {
    if delay_ms > 0 {
        thread::sleep(Duration::from_millis(delay_ms));
    }

    let mut enigo = create_enigo()?;
    clear_modifiers_with(&mut enigo)?;

    send_key(&mut enigo, Key::Control, Direction::Press, "按下 Ctrl")?;
    thread::sleep(Duration::from_millis(KEY_STEP_MS));
    let copy_result = send_key(&mut enigo, Key::Other(0x43), Direction::Click, "发送复制键");
    thread::sleep(Duration::from_millis(KEY_STEP_MS));
    let release_result = send_key(&mut enigo, Key::Control, Direction::Release, "释放 Ctrl");
    copy_result?;
    release_result?;

    Ok(())
}

/// 直接模拟键盘逐字输入纯文本
#[allow(dead_code)]
pub fn type_text(text: &str, delay_ms: u64) -> Result<(), String> {
    if delay_ms > 0 {
        thread::sleep(Duration::from_millis(delay_ms));
    }

    let mut enigo = create_enigo()?;
    clear_modifiers_with(&mut enigo)?;

    enigo
        .text(text)
        .map_err(|e| format!("由于系统错误导致的输入失败: {}", e))?;
    Ok(())
}

/// 模拟全选快捷键 (系统适配版本)
pub fn simulate_select_all() -> Result<(), String> {
    let mut enigo = create_enigo()?;
    clear_modifiers_with(&mut enigo)?;

    #[cfg(target_os = "macos")]
    {
        send_key(&mut enigo, Key::Meta, Direction::Press, "按下 Meta")?;
        thread::sleep(Duration::from_millis(KEY_STEP_MS));
        let select_result = send_key(
            &mut enigo,
            Key::Unicode('a'),
            Direction::Click,
            "发送全选键",
        );
        thread::sleep(Duration::from_millis(KEY_STEP_MS));
        let release_result = send_key(&mut enigo, Key::Meta, Direction::Release, "释放 Meta");
        select_result?;
        release_result?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        send_key(&mut enigo, Key::Control, Direction::Press, "按下 Ctrl")?;
        thread::sleep(Duration::from_millis(KEY_STEP_MS));
        let select_result = send_key(
            &mut enigo,
            Key::Unicode('a'),
            Direction::Click,
            "发送全选键",
        );
        thread::sleep(Duration::from_millis(KEY_STEP_MS));
        let release_result = send_key(&mut enigo, Key::Control, Direction::Release, "释放 Ctrl");
        select_result?;
        release_result?;
    }

    Ok(())
}
