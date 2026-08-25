#[cfg(target_os = "windows")]
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(target_os = "windows")]
const VALUE_NAME: &str = "SnapPaste";

#[cfg(target_os = "windows")]
fn command_value() -> Result<String, String> {
    let executable =
        std::env::current_exe().map_err(|error| format!("无法获取程序路径：{error}"))?;
    Ok(format!("\"{}\" --autostart", executable.display()))
}

#[cfg(target_os = "windows")]
pub fn set_enabled(enabled: bool) -> Result<(), String> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};

    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    let (run_key, _) = current_user
        .create_subkey_with_flags(RUN_KEY, KEY_READ | KEY_WRITE)
        .map_err(|error| format!("无法打开开机启动配置：{error}"))?;

    if enabled {
        run_key
            .set_value(VALUE_NAME, &command_value()?)
            .map_err(|error| format!("无法启用开机启动：{error}"))
    } else {
        match run_key.delete_value(VALUE_NAME) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("无法禁用开机启动：{error}")),
        }
    }
}

#[cfg(target_os = "windows")]
pub fn is_enabled() -> Result<bool, String> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};

    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = match current_user.open_subkey_with_flags(RUN_KEY, KEY_READ) {
        Ok(key) => key,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(format!("无法读取开机启动配置：{error}")),
    };
    Ok(run_key.get_value::<String, _>(VALUE_NAME).is_ok())
}

#[cfg(not(target_os = "windows"))]
pub fn set_enabled(_enabled: bool) -> Result<(), String> {
    Err("当前平台暂不支持开机启动设置".to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn is_enabled() -> Result<bool, String> {
    Ok(false)
}

pub fn sync(enabled: bool) -> Result<(), String> {
    if is_enabled()? == enabled {
        return Ok(());
    }
    set_enabled(enabled)
}
