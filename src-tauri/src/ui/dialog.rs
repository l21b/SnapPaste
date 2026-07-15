use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// 弹窗类型 (前端需匹配 "info", "error", "success")
#[derive(Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DialogType {
    Info,
    Error,
}

/// 序列化传输载体 (使用生命周期实现零拷贝)
#[derive(Clone, Serialize)]
struct DialogMessage<'a> {
    msg_type: DialogType,
    title: &'a str,
    content: &'a str,
}

/// 内部辅助：向前端发送标准的弹窗事件
fn emit_dialog(app: &AppHandle, msg_type: DialogType, title: &str, content: &str) {
    let dialog = DialogMessage {
        msg_type,
        title,
        content,
    };
    if let Err(e) = app.emit("popup-content", dialog) {
        eprintln!("Failed to emit dialog event: {}", e);
    }
}

/// 在主界面显示提示弹窗
pub fn show_popup(app: &AppHandle, msg_type: DialogType, title: &str, content: &str) {
    emit_dialog(app, msg_type, title, content);
}
