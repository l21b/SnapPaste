//! clipboard 模块 - 剪贴板相关功能
//! - clipboard.rs: 底层读写操作
//! - monitor.rs:   剪贴板变化监听守护线程
//! - services.rs:  业务逻辑（快捷键处理、AI 处理、设置保存）

pub mod clipboard;
pub mod monitor;
pub mod processor;
pub mod services;

// Re-export 常用函数，供上层直接调用
pub use clipboard::*;
