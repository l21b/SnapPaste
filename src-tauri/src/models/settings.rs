use serde::{Deserialize, Serialize};

/// 主题类型枚举
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
    System,
}

/// 应用设置结构
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub hotkey: String,   // 全局快捷键
    pub theme: Theme,     // 主题 (强类型)
    pub keep_days: i32,   // 保留天数
    pub max_records: i32, // 最大记录数
    pub auto_start: bool, // 是否开机自启

    // ========== AI ==========
    pub ai_enabled: bool,    // 是否启用 AI
    pub ai_hotkey: String,   // AI 快捷键
    pub ai_api_url: String,  // AI API 地址
    pub ai_api_key: String,  // AI API 密钥
    pub ai_model: String,    // AI 模型
    pub ai_prompt: String,   // AI 提示词
    pub ai_temperature: f32, // AI 温度参数
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "Alt+Z".to_string(),
            theme: Theme::System,
            keep_days: 1,
            max_records: 500,
            auto_start: false,

            // ========== AI ==========
            ai_enabled: true,
            ai_hotkey: "Alt+X".to_string(),
            ai_api_url: "".to_string(),
            ai_api_key: "".to_string(),
            ai_model: "".to_string(),
            ai_prompt: "你是拼音纠错专家。修正输入中的同音/简拼错误，禁止润色，严禁解释，直接输出修正后全文。".to_string(),
            ai_temperature: 0.3,
        }
    }
}
