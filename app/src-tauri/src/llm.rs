// llm.rs — OpenAI-compatible LLM 客户端
// 等价于 Python 的 call_local_llm

use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f64,
    top_p: f64,
    // None 时不发送 max_tokens，让模型生成到自然结束（仅受上下文窗口约束）
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
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
    // 有些「思考模型」会返回 content: null 或空串，把内容放进 reasoning_content
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    reasoning_content: Option<String>,
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
    max_tokens: Option<u32>,
) -> Result<String, String> {
    // 宽容处理：用户常漏写 http:// 协议头，缺失时自动补上，避免 reqwest builder error
    let trimmed = base_url.trim().trim_end_matches('/');
    let normalized = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("http://{}", trimmed)
    };
    let url = format!("{}/chat/completions", normalized);

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

    let msg = match chat_resp.choices.first() {
        Some(c) => &c.message,
        None => return Err("模型返回的 choices 为空，没有生成内容。".to_string()),
    };
    let content = msg.content.clone().unwrap_or_default();
    if !content.trim().is_empty() {
        return Ok(content);
    }
    // content 为空：若只有思考内容，说明是「思考模型」把输出额度用在了 reasoning 上
    if msg
        .reasoning_content
        .as_ref()
        .map(|r| !r.trim().is_empty())
        .unwrap_or(false)
    {
        return Err("模型只返回了思考内容(reasoning)、正文为空。这通常是「思考模型」把输出额度用在了思考上。\
请在「设置」开启『关闭思考』，或增大输出上限 / 换用非思考模型。".to_string());
    }
    Err("模型返回了空内容。".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(max_tokens: Option<u32>) -> String {
        let req = ChatRequest {
            model: "m".into(),
            messages: vec![],
            temperature: 0.5,
            top_p: 0.9,
            max_tokens,
        };
        serde_json::to_string(&req).unwrap()
    }

    #[test]
    fn omits_max_tokens_when_none() {
        // None 时请求体里不应出现 max_tokens（让模型写到自然结束）
        assert!(!build(None).contains("max_tokens"));
        // Some 时应包含
        assert!(build(Some(4096)).contains("\"max_tokens\":4096"));
    }
}
