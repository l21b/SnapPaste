use crate::clipboard;
use crate::keyboard::keyboard;
use std::error::Error;
use std::time::Duration;

#[cfg(target_os = "windows")]
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};

#[cfg(target_os = "windows")]
use windows::Win32::System::DataExchange::GetClipboardSequenceNumber;

#[cfg(target_os = "windows")]
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationTextPattern, UIA_TextPatternId,
};

/// RAII 包装 COM 初始化
#[cfg(target_os = "windows")]
struct ComInitializer {
    should_uninit: bool,
}

#[cfg(target_os = "windows")]
impl ComInitializer {
    fn new() -> Result<Self, Box<dyn Error>> {
        // 使用 CoInitializeEx 并明确模式
        let result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        Ok(Self {
            should_uninit: result.is_ok(),
        })
    }
}

#[cfg(target_os = "windows")]
impl Drop for ComInitializer {
    fn drop(&mut self) {
        if self.should_uninit {
            unsafe { CoUninitialize() };
        }
    }
}

/// 通过 Windows UI 自动化获取焦点元素的文本
#[cfg(target_os = "windows")]
fn get_text_by_automation() -> Result<String, Box<dyn Error>> {
    let _com = ComInitializer::new()?;

    let target = {
        let auto: IUIAutomation = unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL) }?;
        let el = unsafe { auto.GetFocusedElement() }?;

        // 如果无法获取 TextPattern，说明该控件不提供文本自动化支持
        let res: IUIAutomationTextPattern = unsafe { el.GetCurrentPatternAs(UIA_TextPatternId) }?;
        let text_array = unsafe { res.GetSelection() }?;
        let length = unsafe { text_array.Length() }?;

        let mut target = String::new();
        for i in 0..length {
            let text = unsafe { text_array.GetElement(i) }?;
            let str = unsafe { text.GetText(-1) }?;
            target.push_str(&str.to_string());
        }
        target
    };

    Ok(target.trim().to_string())
}

/// 通过模拟 Ctrl+C 获取选中文本，只处理文本
#[cfg(target_os = "windows")]
fn get_text_by_clipboard() -> Result<String, Box<dyn Error>> {
    // 使用单一 Context 避免频繁创建回收连接
    let mut ctx =
        clipboard::ClipboardContext::new().map_err(|e| format!("无法初始化剪贴板: {}", e))?;

    // 1. 备份现有剪贴板内容
    let old_text = match ctx.read_text() {
        Ok(text) => Some(text),
        Err(_) => None, // 如果原来没文本，就记录为 None
    };

    let num_before = unsafe { GetClipboardSequenceNumber() };

    // 2. 调用键盘基础设施模拟复制
    keyboard::simulate_copy(0).map_err(|e| format!("模拟复制失败: {}", e))?;

    // 3. 轮询等待剪贴板变化
    let start = std::time::Instant::now();
    let mut copy_success = false;
    while start.elapsed() < Duration::from_millis(300) {
        if unsafe { GetClipboardSequenceNumber() } != num_before {
            copy_success = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    if !copy_success {
        return Err("Copy failed or timeout".into());
    }

    // 4. 读取新复制进来的文本
    let new_text_result = ctx.read_text();

    // 5. 无论刚复制的是什么，先尝试恢复原文本
    if let Some(text) = old_text {
        let _ = ctx.write_text(&text);
    }

    // 6. 处理并返回结果
    let new_text = new_text_result.map_err(|e| format!("读取新剪贴板失败: {}", e))?;
    let result = new_text.trim().to_string();

    if result.is_empty() {
        Err("Copied text is empty".into())
    } else {
        Ok(result)
    }
}

/// 获取当前选中的文本（Windows 专用），优先使用 UI 自动化 API，回退到剪贴板方式。
#[cfg(target_os = "windows")]
pub fn get_selected_text() -> String {
    if let Ok(text) = get_text_by_automation() {
        if !text.is_empty() {
            return text;
        }
    }

    if let Ok(text) = get_text_by_clipboard() {
        if !text.is_empty() {
            return text;
        }
    }

    String::new()
}

/// 非 Windows 平台返回空字符串
#[cfg(not(target_os = "windows"))]
pub fn get_selected_text() -> String {
    String::new()
}
