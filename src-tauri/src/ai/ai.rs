use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// AI 调用配置参数
pub struct AiConfig<'a> {
    pub api_url: &'a str,
    pub api_key: &'a str,
    pub model: &'a str,
    pub prompt: &'a str,
    pub temperature: f32,
}

/// OpenAI 聊天补全请求体
#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    temperature: f32,
}

/// 聊天消息结构
#[derive(Serialize)]
struct ChatMessage<'a> {
    /// 角色：system（系统）、user（用户）、assistant（AI）
    role: &'a str,
    /// 消息内容
    content: String,
}

/// OpenAI 聊天补全响应体
#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

/// 响应中的单个选择
#[derive(Deserialize)]
struct Choice {
    /// AI 生成的消息
    message: Message,
}

/// AI 生成的消息内容
#[derive(Deserialize)]
struct Message {
    content: String,
}

/// 创建全局复用的 HTTP 客户端
pub fn create_ai_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .tcp_keepalive(Some(Duration::from_secs(60)))
        .build()
        .map_err(|e| format!("初始化 HTTP 客户端失败: {}", e))
}

/// 调用 AI API 处理文本
pub async fn process_text(client: &Client, config: &AiConfig<'_>, text: &str) -> Result<String, String> {
    // 1. 构建完整的 API URL
    let full_url = if config.api_url.ends_with("/chat/completions") {
        // 如果用户已提供完整 URL，直接使用
        config.api_url.to_string()
    } else {
        // 否则自动追加 /chat/completions 后缀
        format!("{}/chat/completions", config.api_url.trim_end_matches('/'))
    };

    // 2. 文本长度检查与截断
    const MAX_CHARS: usize = 10000;

    // 使用 char_indices() 按字符而非字节截断（避免 UTF-8 截断问题）
    let final_content = if let Some((byte_index, _)) = text.char_indices().nth(MAX_CHARS) {
        format!(
            "{}\n\n{}...\n[内容过长已截断]",
            config.prompt,
            &text[..byte_index]
        )
    } else {
        format!("{}\n\n{}", config.prompt, text)
    };

    // 3. 构建请求体
    let request = ChatRequest {
        model: config.model,
        messages: vec![ChatMessage {
            role: "user",
            content: final_content,
        }],
        temperature: config.temperature,
    };

    // 4. 发送 HTTP POST 请求
    let response = client
        .post(&full_url)
        .bearer_auth(config.api_key)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("网络连接失败: {}", e))?;

    // 5. 检查 HTTP 状态码
    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().await.unwrap_or_default();
        return Err(format!(
            "API 报错 (状态码 {}): {}",
            status, error_body
        ));
    }

    // 6. 解析 JSON 响应
    let chat_response: ChatResponse = response
        .json()
        .await
        .map_err(|e| format!("JSON 解析失败: {}", e))?;

    // 7. 提取 AI 生成的内容
    chat_response
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| "AI 返回了空结果".to_string())
        .map(|c| c.message.content.trim().to_string())
}

/// 便捷入口：从应用设置直接调用 AI 处理
pub async fn process_text_with_settings(
    client: &Client,
    settings: &crate::models::Settings,
    text: &str,
) -> Result<String, String> {
    if !settings.ai_enabled {
        return Err("AI 功能未启用".to_string());
    }
    if settings.ai_api_url.is_empty() || settings.ai_api_key.is_empty() {
        return Err("AI 设置未配置".to_string());
    }

    let config = AiConfig {
        api_url: &settings.ai_api_url,
        api_key: &settings.ai_api_key,
        model: &settings.ai_model,
        prompt: &settings.ai_prompt,
        temperature: settings.ai_temperature,
    };

    process_text(client, &config, text).await
}
