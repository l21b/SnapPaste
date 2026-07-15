use std::collections::HashMap;
use std::sync::RwLock;
use tauri::{AppHandle, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

struct Inner {
    shortcuts: HashMap<String, Shortcut>,
}

/// 快捷键管理器，负责应用中所有全局快捷键的注册、注销和路由转发
pub struct HotkeyManager {
    inner: RwLock<Inner>,
}

impl HotkeyManager {
    /// 创建一个新的快捷键管理器实例
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Inner {
                shortcuts: HashMap::new(),
            }),
        }
    }

    /// 注册一个全局快捷键
    /// - `id`: 为该快捷键分配的唯一标识符（例如 "main" 或 "ai"）
    /// - `shortcut_str`: 快捷键字符串标识（例如 "CommandOrControl+Shift+V"）
    pub fn register<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        id: &str,
        shortcut_str: &str,
    ) -> Result<(), String> {
        // 解析快捷键字符串
        let new_shortcut: Shortcut = shortcut_str
            .trim()
            .parse()
            .map_err(|e| format!("无效快捷键格式: {}", e))?;

        let mut inner = self.inner.write().map_err(|_| "锁已损坏".to_string())?;
        let manager = app.global_shortcut();

        // 避免重复注册相同的快捷键
        if inner.shortcuts.get(id) == Some(&new_shortcut) {
            return Ok(());
        }

        // 向操作系统注册新的全局快捷键
        manager
            .register(new_shortcut)
            .map_err(|e| format!("注册冲突或失败: {}", e))?;

        // 只有旧快捷键成功注销后才更新内存状态；失败时撤销新注册。
        if let Some(old_shortcut) = inner.shortcuts.get(id).cloned()
            && let Err(error) = manager.unregister(old_shortcut)
        {
            let _ = manager.unregister(new_shortcut);
            return Err(format!("无法注销旧快捷键: {}", error));
        }
        inner.shortcuts.insert(id.to_string(), new_shortcut);

        Ok(())
    }

    /// 注销指定 ID 的全局快捷键
    pub fn unregister<R: Runtime>(&self, app: &AppHandle<R>, id: &str) -> Result<(), String> {
        let mut inner = self.inner.write().map_err(|_| "锁已损坏".to_string())?;
        let manager = app.global_shortcut();

        // 从内部映射中移除并取消系统注册
        if let Some(old_shortcut) = inner.shortcuts.get(id).cloned() {
            manager
                .unregister(old_shortcut)
                .map_err(|e| format!("无法注销快捷键: {}", e))?;
            inner.shortcuts.remove(id);
        }
        Ok(())
    }

    /// 快捷键路由转发
    /// 当系统触发全局快捷键事件时，根据该 Shortcut 实例寻找其对应的字符串 ID
    pub fn route(&self, shortcut: &Shortcut) -> Option<String> {
        let inner = self.inner.read().ok()?;
        inner
            .shortcuts
            .iter()
            .find(|(_, s)| *s == shortcut)
            .map(|(id, _)| id.clone())
    }
}
