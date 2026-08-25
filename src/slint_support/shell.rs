fn normalize_web_url(value: &str) -> Result<String, String> {
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

#[cfg(target_os = "windows")]
pub fn open_url(value: &str) -> Result<(), String> {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let url = normalize_web_url(value)?;
    let wide_url: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
    let operation: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
    let result = unsafe {
        ShellExecuteW(
            null_mut(),
            operation.as_ptr(),
            wide_url.as_ptr(),
            null(),
            null(),
            SW_SHOWNORMAL,
        )
    };
    if result as isize <= 32 {
        return Err(format!("无法打开链接（系统错误码：{}）", result as isize));
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn open_url(value: &str) -> Result<(), String> {
    let url = normalize_web_url(value)?;
    let (program, argument) = if cfg!(target_os = "macos") {
        ("open", url)
    } else {
        ("xdg-open", url)
    };
    std::process::Command::new(program)
        .arg(argument)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("无法打开链接：{error}"))
}

#[cfg(target_os = "windows")]
pub fn system_uses_dark_theme() -> bool {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};

    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(key) = current_user.open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
        KEY_READ,
    ) else {
        return false;
    };
    key.get_value::<u32, _>("AppsUseLightTheme")
        .map(|value| value == 0)
        .unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
pub fn system_uses_dark_theme() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::normalize_web_url;

    #[test]
    fn normalizes_web_urls() {
        assert_eq!(
            normalize_web_url("www.example.com").expect("normalize www URL"),
            "https://www.example.com"
        );
        assert!(normalize_web_url("file:///tmp/private").is_err());
    }
}
