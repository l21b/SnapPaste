use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

use crate::clipboard::clipboard::ClipboardContext;
use crate::db::queries::add_record; 
use crate::models::{ClipboardRecord, ContentType};
use crate::utils::image::{
    encode_rgba_to_png, image_signature, normalize_image_for_storage, MAX_ENCODED_IMAGE_BYTES,
    MAX_IMAGE_BYTES,
};

// ==========================================
// 策略配置
// ==========================================

const ENABLE_IMAGE_RECORDING: bool = false;
const MIN_IMAGE_RECORD_INTERVAL_MS: u64 = 1200;

static LAST_IMAGE_RECORD_MS: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, PartialEq, Eq)]
pub enum ClipboardSignature {
    None,
    Text(u64),
    Image(String),
}

static LAST_SIGNATURE: Mutex<ClipboardSignature> = Mutex::new(ClipboardSignature::None);

// ==========================================
// 公共检测接口
// ==========================================

pub fn identify_text_type(text: &str) -> ContentType {
    let trimmed = text.trim();
    if trimmed.is_empty() { return ContentType::Text; }

    let lower = trimmed.to_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("www.") {
        return ContentType::Link;
    }
    ContentType::Text
}

pub fn compute_text_hash(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

// ==========================================
// 核心处理逻辑
// ==========================================

pub fn init_startup_signature() -> Result<(), String> {
    let mut ctx = ClipboardContext::new().map_err(|e| format!("Clipboard Init: {:?}", e))?;

    if let Ok(text) = ctx.read_text() {
        let sig = compute_text_hash(&text);
        *LAST_SIGNATURE.lock().unwrap() = ClipboardSignature::Text(sig);
        return Ok(());
    }

    if ENABLE_IMAGE_RECORDING {
        if let Ok(image) = ctx.read_image() {
            let sig = image_signature(image.width, image.height, &image.bytes);
            *LAST_SIGNATURE.lock().unwrap() = ClipboardSignature::Image(sig);
        }
    }
    Ok(())
}

pub fn process_clipboard_change(app: &AppHandle) -> Result<(), String> {
    let mut ctx = ClipboardContext::new().map_err(|e| format!("Clipboard Context: {:?}", e))?;

    // 在很多复合复制场景（如 Excel/浏览器）中，文本的价值往往高于附带的截图。
    if let Ok(text) = ctx.read_text() {
        if !text.trim().is_empty() {
            try_process_text_change(text, app);
            return Ok(());
        }
    }

    // 只有当剪贴板里完全没有有效文本时，才去尝试抓取纯图片（例如使用截图工具）
    if ENABLE_IMAGE_RECORDING {
        if let Ok(image) = ctx.read_image() {
            try_process_image_change(image.width, image.height, image.bytes, app);
        }
    }

    Ok(())
}

/// 针对图片变化的细分处理
fn try_process_image_change(width: usize, height: usize, raw: Vec<u8>, app: &AppHandle) {
    if raw.len() > MAX_IMAGE_BYTES { return; }

    let signature = image_signature(width, height, &raw);

    // 指纹查重 (极速内存操作)
    {
        let mut last_sig = LAST_SIGNATURE.lock().unwrap();
        if let ClipboardSignature::Image(ref last_img_sig) = *last_sig {
            if last_img_sig == &signature { return; }
        }
        *last_sig = ClipboardSignature::Image(signature);
    }

    // 频率节流检查
    let now = now_ms();
    let last = LAST_IMAGE_RECORD_MS.load(Ordering::SeqCst);
    if now.saturating_sub(last) < MIN_IMAGE_RECORD_INTERVAL_MS { return; }
    LAST_IMAGE_RECORD_MS.store(now, Ordering::SeqCst);

    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        match build_image_record(width, height, &raw) {
            Ok(record) => {
                persist_record_and_emit(&app_clone, record);
            }
            Err(e) => eprintln!("[Processor] Image encoding failed: {}", e),
        }
    });
}

fn try_process_text_change(text: String, app: &AppHandle) {
    if text.trim().is_empty() { return; }

    let signature = compute_text_hash(&text);

    {
        let mut last_sig = LAST_SIGNATURE.lock().unwrap();
        if let ClipboardSignature::Text(last_txt_sig) = *last_sig {
            if last_txt_sig == signature { return; }
        }
        *last_sig = ClipboardSignature::Text(signature);
    }

    // 文本处理极快，直接在当前线程完成即可
    let record = build_record_from_text(text);
    persist_record_and_emit(app, record);
}

// ==========================================
// 辅助模型构建
// ==========================================

pub fn build_record_from_text(text: String) -> ClipboardRecord {
    let content_type = identify_text_type(&text);
    ClipboardRecord {
        id: 0,
        content_type,
        content: text,
        image_data: None,
        is_favorite: false,
        is_pinned: false,
        created_at: chrono::Local::now().to_rfc3339(),
    }
}

pub fn build_image_record(width: usize, height: usize, rgba: &[u8]) -> Result<ClipboardRecord, String> {
    let (normalized_width, normalized_height, normalized_rgba, scaled) =
        normalize_image_for_storage(width, height, rgba);

    let png_bytes = encode_rgba_to_png(
        normalized_width,
        normalized_height,
        normalized_rgba.as_ref(),
    )?;

    if png_bytes.len() > MAX_ENCODED_IMAGE_BYTES {
        return Err(format!("Image too large after encoding ({} bytes)", png_bytes.len()));
    }

    let description = if scaled {
        format!("图片 {}x{} (缩放自 {}x{})", normalized_width, normalized_height, width, height)
    } else {
        format!("图片 {}x{}", width, height)
    };

    Ok(ClipboardRecord {
        id: 0,
        content_type: ContentType::Image,
        content: description,
        image_data: Some(png_bytes),
        is_favorite: false,
        is_pinned: false,
        created_at: chrono::Local::now().to_rfc3339(),
    })
}

fn persist_record_and_emit(app: &AppHandle, record: ClipboardRecord) -> bool {
    // 调用重构后的事务安全的 add_record
    if let Err(e) = add_record(record) {
        eprintln!("[Processor] DB Write Error: {:?}", e);
        false
    } else {
        let _ = app.emit("history-changed", ());
        true
    }
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}