//! clipboard 模块 - 剪贴板相关功能
//! - access.rs:    底层读写操作
//! - monitor.rs:   剪贴板变化监听守护线程
//!
//! Slint 桌面层通过这些与 UI 无关的接口访问剪贴板。

pub mod access;
pub mod monitor;
pub mod processor;
