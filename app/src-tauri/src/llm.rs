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

// ============================================================
// 订阅 CLI 路线：Claude Code / Codex
//
// 这两条路线不走 HTTP API，而是把本机已登录订阅账号的官方 CLI
// （`claude` / `codex`）当作子进程调用，从而走 Pro/Max / ChatGPT 订阅额度，
// 而不是按量计费的 API key。提示词通过 stdin 传入（避免命令行长度/转义限制），
// 结果从 stdout(JSON) 或 --output-last-message 文件读取。
// ============================================================

/// 取字符串前 n 个字符（用于错误信息里截断原始输出）。
fn clip_head(s: &str, n: usize) -> String {
    s.trim().chars().take(n).collect()
}

/// 取字符串末 n 个字符（CLI 报错通常在 stderr 末尾）。
fn clip_tail(s: &str, n: usize) -> String {
    let t = s.trim();
    let count = t.chars().count();
    if count <= n {
        t.to_string()
    } else {
        t.chars().skip(count - n).collect()
    }
}

/// 在 PATH 中解析可执行文件，返回 (路径, 是否为批处理脚本)。
/// 批处理脚本(.cmd/.bat)无法被 CreateProcess 直接执行，需经 `cmd /C` 调用。
#[cfg(windows)]
fn resolve_executable(name: &str) -> Option<(std::path::PathBuf, bool)> {
    use std::path::Path;
    // 用户直接给了存在的路径
    let direct = Path::new(name);
    if direct.is_file() {
        let is_batch = matches!(
            direct
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_ascii_lowercase())
                .as_deref(),
            Some("cmd") | Some("bat")
        );
        return Some((direct.to_path_buf(), is_batch));
    }
    let path_var = std::env::var_os("PATH")?;
    // 优先 .exe（可直接执行，能原样传多行参数）；其次 npm 的 .cmd/.bat 垫片
    for ext in [".exe", ".cmd", ".bat"] {
        for dir in std::env::split_paths(&path_var) {
            let cand = dir.join(format!("{}{}", name, ext));
            if cand.is_file() {
                return Some((cand, ext != ".exe"));
            }
        }
    }
    None
}

/// 非 Windows：交给 PATH 解析（Command::new 会查找 PATH），不存在批处理垫片问题。
#[cfg(not(windows))]
fn resolve_executable(name: &str) -> Option<(std::path::PathBuf, bool)> {
    use std::path::Path;
    let direct = Path::new(name);
    if direct.is_file() {
        return Some((direct.to_path_buf(), false));
    }
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let cand = dir.join(name);
        if cand.is_file() {
            return Some((cand, false));
        }
    }
    // 兜底：直接用名字，让 Command 自行解析
    Some((std::path::PathBuf::from(name), false))
}

/// 以子进程方式运行 CLI：把 stdin_data 写入 stdin，捕获 stdout/stderr。
/// - env_remove: 从子进程环境中删除的变量（用于强制走订阅而非 API key）。
/// - 用独立线程喂 stdin、读 stdout/stderr，避免管道缓冲区写满造成死锁。
/// 返回 (exit_code, stdout, stderr)。
fn run_cli(
    resolved: &(std::path::PathBuf, bool),
    args: &[String],
    env_remove: &[&str],
    stdin_data: &str,
    timeout_secs: u64,
) -> Result<(Option<i32>, String, String), String> {
    use std::io::{Read, Write};
    use std::process::Stdio;

    let (path, is_batch) = resolved;

    let mut cmd = if *is_batch {
        // cmd /C <批处理> <参数...>
        let mut c = std::process::Command::new("cmd");
        c.arg("/C").arg(path);
        c
    } else {
        std::process::Command::new(path)
    };
    cmd.args(args);
    for key in env_remove {
        cmd.env_remove(key);
    }

    // 中性工作目录：避免 claude 加载小说项目/仓库里的 CLAUDE.md，或 codex 误判 git 仓库
    let work = std::env::temp_dir().join("sodarie_cli_work");
    let _ = std::fs::create_dir_all(&work);
    cmd.current_dir(&work);

    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("无法启动 {} 进程: {}", path.display(), e))?;

    // 喂 stdin（独立线程；写完 drop 关闭管道 → EOF）
    let mut stdin = child.stdin.take().ok_or("无法获取子进程 stdin")?;
    let input = stdin_data.to_string();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(input.as_bytes());
    });

    // 读 stdout / stderr（独立线程，防止缓冲区写满死锁）
    let mut out = child.stdout.take().ok_or("无法获取子进程 stdout")?;
    let out_handle = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = out.read_to_string(&mut s);
        s
    });
    let mut err = child.stderr.take().ok_or("无法获取子进程 stderr")?;
    let err_handle = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = err.read_to_string(&mut s);
        s
    });

    // 轮询等待，带超时
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) => {
                if std::time::Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "调用 {} 超时（> {}s），已终止子进程。",
                        path.display(),
                        timeout_secs
                    ));
                }
                std::thread::sleep(std::time::Duration::from_millis(150));
            }
            Err(e) => return Err(format!("等待子进程失败: {}", e)),
        }
    };

    let _ = writer.join();
    let stdout = out_handle.join().unwrap_or_default();
    let stderr = err_handle.join().unwrap_or_default();
    Ok((status.code(), stdout, stderr))
}

/// 从 claude `--output-format json` 输出里解析 (is_error, result_text)。
fn extract_claude_result(stdout: &str) -> Option<(bool, String)> {
    let parse = |s: &str| -> Option<(bool, String)> {
        let v: serde_json::Value = serde_json::from_str(s.trim()).ok()?;
        let result = v.get("result").and_then(|x| x.as_str())?.to_string();
        let is_err = v.get("is_error").and_then(|x| x.as_bool()).unwrap_or(false);
        Some((is_err, result))
    };
    let trimmed = stdout.trim();
    if let Some(r) = parse(trimmed) {
        return Some(r);
    }
    // 兜底：从下往上扫描，找到第一行能解析出 result 的 JSON 对象
    for line in trimmed.lines().rev() {
        let l = line.trim();
        if l.starts_with('{') {
            if let Some(r) = parse(l) {
                return Some(r);
            }
        }
    }
    None
}

/// 通过本机已登录的 Claude Code 订阅调用模型（非 API key 计费）。
/// - model 留空则使用订阅默认模型；否则按别名/全名传给 `--model`（如 sonnet/opus）。
/// - 用户提示词经 stdin 传入；系统提示词经 `--system-prompt` 传入（替换默认编码助手提示词）。
pub fn call_claude_code(model: &str, system: &str, user: &str) -> Result<String, String> {
    let resolved = resolve_executable("claude").ok_or_else(|| {
        "未找到 claude 命令。请先安装 Claude Code 并用订阅账号登录：\n\
         1) 安装：https://claude.com/claude-code （或 npm i -g @anthropic-ai/claude-code）\n\
         2) 登录订阅：在终端运行 `claude`，按提示用 Pro/Max 账号登录后再使用本选项。"
            .to_string()
    })?;

    let mut args: Vec<String> = vec![
        "-p".into(),
        "--output-format".into(),
        "json".into(),
    ];
    if !system.trim().is_empty() {
        args.push("--system-prompt".into());
        args.push(system.to_string());
    }
    let m = model.trim();
    if !m.is_empty() {
        args.push("--model".into());
        args.push(m.to_string());
    }

    // 删除 ANTHROPIC_API_KEY/TOKEN，强制走 OAuth 订阅而非按量计费
    let (code, stdout, stderr) = run_cli(
        &resolved,
        &args,
        &["ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN"],
        user,
        1800,
    )?;

    match extract_claude_result(&stdout) {
        Some((false, text)) if !text.trim().is_empty() => Ok(text.trim().to_string()),
        Some((true, text)) => Err(format!(
            "claude 返回错误：{}",
            clip_head(&text, 800)
        )),
        Some((false, _)) => Err(format!(
            "claude 未返回正文内容。\nstderr:\n{}",
            clip_tail(&stderr, 800)
        )),
        None => {
            if code != Some(0) {
                Err(format!(
                    "claude 调用失败（退出码 {:?}）。请确认已用订阅账号登录\
                     （在终端运行 `claude` 登录 Pro/Max，且未设置 ANTHROPIC_API_KEY）。\n\
                     stderr:\n{}",
                    code,
                    clip_tail(&stderr, 800)
                ))
            } else {
                Err(format!(
                    "解析 claude 输出失败。\nstdout:\n{}\nstderr:\n{}",
                    clip_head(&stdout, 800),
                    clip_tail(&stderr, 800)
                ))
            }
        }
    }
}

/// 通过本机已登录的 Codex（ChatGPT 订阅）调用模型（非 API key 计费）。
/// - model 留空则使用 codex 默认模型；否则传给 `-m`（如 gpt-5-codex）。
/// - codex 无独立系统提示词参数，这里把 system+user 合并为一段 prompt 经 stdin 传入。
/// - 最终回答从 `--output-last-message` 临时文件读取（干净，不含过程日志）。
pub fn call_codex(model: &str, system: &str, user: &str) -> Result<String, String> {
    let resolved = resolve_executable("codex").ok_or_else(|| {
        "未找到 codex 命令。请先安装 Codex CLI 并用 ChatGPT 订阅登录：\n\
         1) 安装：npm i -g @openai/codex\n\
         2) 登录订阅：在终端运行 `codex login`（选择 Sign in with ChatGPT）后再使用本选项。"
            .to_string()
    })?;

    let prompt = if system.trim().is_empty() {
        user.to_string()
    } else {
        format!("{}\n\n{}", system, user)
    };

    // 最终回答输出文件（codex 会把最后一条 agent 消息写入此文件）
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let out_path = std::env::temp_dir().join(format!(
        "sodarie_codex_{}_{}.txt",
        std::process::id(),
        nanos
    ));
    let out_str = out_path.to_string_lossy().to_string();

    let mut args: Vec<String> = vec![
        "exec".into(),
        "--skip-git-repo-check".into(),
        "--sandbox".into(),
        "read-only".into(),
        "--ephemeral".into(),
        "--output-last-message".into(),
        out_str,
    ];
    let m = model.trim();
    if !m.is_empty() {
        args.push("-m".into());
        args.push(m.to_string());
    }
    // `-` 表示从 stdin 读取指令
    args.push("-".into());

    // 删除 OPENAI_API_KEY，强制走 ChatGPT 订阅而非按量计费
    let result = run_cli(&resolved, &args, &["OPENAI_API_KEY"], &prompt, 1800);

    let final_msg = std::fs::read_to_string(&out_path).unwrap_or_default();
    let _ = std::fs::remove_file(&out_path);
    let final_msg = final_msg.trim().to_string();

    let (code, _stdout, stderr) = result?;

    if !final_msg.is_empty() {
        return Ok(final_msg);
    }
    if code != Some(0) {
        return Err(format!(
            "codex 调用失败（退出码 {:?}）。请确认已用 ChatGPT 订阅登录\
             （在终端运行 `codex login`，且未设置 OPENAI_API_KEY）。\nstderr:\n{}",
            code,
            clip_tail(&stderr, 800)
        ));
    }
    Err(format!(
        "codex 未返回内容。\nstderr:\n{}",
        clip_tail(&stderr, 800)
    ))
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

    #[test]
    fn parses_claude_json_result() {
        // 正常成功：取 result
        let ok = r#"{"type":"result","subtype":"success","is_error":false,"result":"第一章正文","session_id":"x"}"#;
        assert_eq!(extract_claude_result(ok), Some((false, "第一章正文".to_string())));
        // 出错：is_error=true
        let err = r#"{"type":"result","is_error":true,"result":"额度用尽"}"#;
        assert_eq!(extract_claude_result(err), Some((true, "额度用尽".to_string())));
        // 前面有杂讯行时，从下往上找到 JSON 行
        let noisy = "warming up...\n{\"is_error\":false,\"result\":\"正文\"}";
        assert_eq!(extract_claude_result(noisy), Some((false, "正文".to_string())));
        // 完全不是 JSON
        assert_eq!(extract_claude_result("not json"), None);
    }

    #[test]
    fn clip_helpers_respect_char_boundaries() {
        // 按字符（非字节）截断，中文不会被切坏
        assert_eq!(clip_head("一二三四五", 2), "一二");
        assert_eq!(clip_tail("一二三四五", 2), "四五");
        assert_eq!(clip_tail("短", 5), "短");
    }
}
