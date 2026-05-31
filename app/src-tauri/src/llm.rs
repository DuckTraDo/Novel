// llm.rs — OpenAI-compatible LLM 客户端
// 等价于 Python 的 call_local_llm

use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f64,
    top_p: f64,
    max_tokens: u32,
}

#[derive(Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessageContent,
}

#[derive(Deserialize)]
struct ChatMessageContent {
    content: String,
}

/// 调用 OpenAI 兼容的 chat/completions 接口
///
/// - base_url: 如 http://localhost:18083/v1（已含 /v1）
/// - api_key: 放 Authorization: Bearer
/// - model: 模型名
/// - messages: OpenAI 格式消息列表
/// - temperature / top_p / max_tokens: 采样参数
pub fn call_llm(
    base_url: &str,
    api_key: &str,
    model: &str,
    messages: Vec<ChatMessage>,
    temperature: f64,
    top_p: f64,
    max_tokens: u32,
) -> Result<String, String> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let body = ChatRequest {
        model: model.to_string(),
        messages,
        temperature,
        top_p,
        max_tokens,
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send();

    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            if e.is_timeout() {
                return Err(format!("连接模型服务超时（{}）。请确认服务正在运行。", url));
            }
            if e.is_connect() {
                return Err(format!(
                    "无法连接模型服务（{}）。请检查 LLM Base URL 是否正确、服务是否启动。",
                    url
                ));
            }
            return Err(format!("请求模型服务失败: {}", e));
        }
    };

    let status = resp.status();
    if !status.is_success() {
        let body_text = resp.text().unwrap_or_default();
        return Err(format!(
            "模型服务返回错误 HTTP {}: {}",
            status.as_u16(),
            body_text.chars().take(500).collect::<String>()
        ));
    }

    let chat_resp: ChatResponse = resp
        .json()
        .map_err(|e| format!("解析模型响应 JSON 失败: {}", e))?;

    chat_resp
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .ok_or_else(|| "模型返回的 choices 为空，没有生成内容。".to_string())
}
