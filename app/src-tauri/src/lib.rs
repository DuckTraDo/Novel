// 小说写作台 Tauri 后端（Rust 原生实现）
// 所有流水线逻辑已从 Python 移植到 Rust，不再依赖 Python 进程。

mod llm;

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::Manager;

// ============================================================
// 数据结构
// ============================================================

#[derive(Serialize, Deserialize, Clone)]
pub struct CommandResult {
    pub success: bool,
    pub log: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub project_dir: String,
    pub llm_base_url: String,
    pub llm_api_key: String,
    pub llm_model: String,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub max_output_tokens: Option<u32>,
    // 关闭模型思考（Qwen3 等思考模型）。None/true = 关闭思考（给 prompt 追加 /no_think）
    pub disable_thinking: Option<bool>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            project_dir: String::new(),
            llm_base_url: "http://localhost:18083/v1".to_string(),
            llm_api_key: "local".to_string(),
            llm_model: String::new(),
            temperature: None,
            top_p: None,
            max_output_tokens: None,
            disable_thinking: None,
        }
    }
}

/// 是否关闭思考：默认开启关闭（None 视为 true），除非用户显式设为 false。
fn thinking_disabled(settings: &Settings) -> bool {
    settings.disable_thinking != Some(false)
}

/// 需要时给 user prompt 追加 Qwen3 的 /no_think 软开关，避免思考吃掉输出额度。
fn apply_no_think(prompt: String, settings: &Settings) -> String {
    if thinking_disabled(settings) {
        format!("{}\n\n/no_think", prompt)
    } else {
        prompt
    }
}

/// 内部运行时配置（从 Settings + config.yaml 合并而来）
struct RunConfig {
    project_dir: PathBuf,
    llm_base_url: String,
    llm_api_key: String,
    model_name: String,
    max_output_tokens: Option<u32>,
}

// ============================================================
// 项目目录 + 设置
// ============================================================

// 设置统一存放在固定的全局位置（应用数据目录），与项目目录解耦，
// 避免「读 / 写位置不一致」导致自定义项目目录时配置丢失。
fn settings_path(app: &tauri::AppHandle) -> PathBuf {
    let app_data = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    app_data.join(".ui-settings.json")
}

fn load_settings(app: &tauri::AppHandle) -> Settings {
    match fs::read_to_string(settings_path(app)) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

fn resolve_project_dir(app: &tauri::AppHandle) -> PathBuf {
    // 先尝试读已有设置
    let app_data = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let default_dir = app_data.join("novel-project");

    // 从全局设置读取自定义项目目录（如果有）
    let settings = load_settings(app);
    let dir = if settings.project_dir.trim().is_empty() {
        default_dir
    } else {
        PathBuf::from(&settings.project_dir)
    };

    // 确保目录存在
    let _ = fs::create_dir_all(&dir);
    dir
}

fn ensure_project_initialized(project_dir: &Path, app: &tauri::AppHandle) {
    let marker = project_dir.join(".initialized");
    if marker.exists() {
        return;
    }

    // 查找 templates 资源目录
    if let Ok(resource_dir) = app.path().resource_dir() {
        let tpl = resource_dir.join("templates");
        if tpl.exists() {
            // 复制 memory 模板
            let tpl_memory = tpl.join("memory");
            let dst_memory = project_dir.join("memory");
            if tpl_memory.exists() {
                let _ = fs::create_dir_all(&dst_memory);
                copy_dir_contents(&tpl_memory, &dst_memory);
            }
            // 复制 outlines 模板
            let tpl_outlines = tpl.join("outlines");
            let dst_outlines = project_dir.join("outlines");
            if tpl_outlines.exists() {
                let _ = fs::create_dir_all(&dst_outlines);
                copy_dir_contents(&tpl_outlines, &dst_outlines);
            }
            // 复制 config.yaml
            let tpl_config = tpl.join("config.yaml");
            if tpl_config.exists() {
                let _ = fs::copy(&tpl_config, project_dir.join("config.yaml"));
            }
        }
    }

    // 确保必要子目录存在
    let _ = fs::create_dir_all(project_dir.join("chapters"));
    let _ = fs::create_dir_all(project_dir.join("outputs").join("reports"));
    // 写初始化标记
    let _ = fs::write(marker, "initialized");
}

fn copy_dir_contents(src: &Path, dst: &Path) {
    let entries = match fs::read_dir(src) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let src_path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);
        if src_path.is_dir() {
            let _ = fs::create_dir_all(&dst_path);
            copy_dir_contents(&src_path, &dst_path);
        } else {
            let _ = fs::copy(&src_path, &dst_path);
        }
    }
}

// ============================================================
// config.yaml 辅助
// ============================================================

fn load_config_yaml(project_dir: &Path) -> serde_yaml::Value {
    let path = project_dir.join("config.yaml");
    match fs::read_to_string(&path) {
        Ok(text) => serde_yaml::from_str(&text).unwrap_or(serde_yaml::Value::Null),
        Err(_) => serde_yaml::Value::Null,
    }
}

fn config_string(config: &serde_yaml::Value, key: &str, default: &str) -> String {
    config
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or(default)
        .to_string()
}

/// 构建运行时配置
fn build_run_config(app: &tauri::AppHandle) -> Result<RunConfig, String> {
    let project_dir = resolve_project_dir(app);
    ensure_project_initialized(&project_dir, app);
    let settings = load_settings(app);
    let config = load_config_yaml(&project_dir);

    let model_name = if settings.llm_model.trim().is_empty() {
        config_string(&config, "model_name", "default-model")
    } else {
        settings.llm_model.trim().to_string()
    };

    // 输出上限：默认不限（None）——只受模型上下文窗口约束。
    // 仅当用户在设置里显式填了正数时才作为上限；填 0 同样视为不限。
    let max_output_tokens = match settings.max_output_tokens {
        Some(n) if n > 0 => Some(n),
        _ => None,
    };

    Ok(RunConfig {
        project_dir,
        llm_base_url: settings.llm_base_url,
        llm_api_key: settings.llm_api_key,
        model_name,
        max_output_tokens,
    })
}

// ============================================================
// 文件 I/O 工具函数
// ============================================================

const EDITABLE_FILES: &[&str] = &[
    "memory/story_bible.yaml",
    "memory/characters.yaml",
    "memory/foreshadowing.yaml",
    "memory/relationships.json",
    "memory/timeline.jsonl",
    "memory/events.jsonl",
    "memory/chapter_summaries.jsonl",
    "memory/style_bank.jsonl",
    "outlines/book_outline.yaml",
    "config.yaml",
];

fn normalize_rel(rel: &str) -> String {
    rel.replace('\\', "/")
}

fn load_yaml(path: &Path) -> serde_yaml::Value {
    match fs::read_to_string(path) {
        Ok(text) => serde_yaml::from_str(&text).unwrap_or(serde_yaml::Value::Mapping(serde_yaml::Mapping::new())),
        Err(_) => serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
    }
}

fn save_yaml(path: &Path, data: &serde_yaml::Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let text = serde_yaml::to_string(data).unwrap_or_default();
    let _ = fs::write(path, text);
}

fn load_json(path: &Path) -> serde_json::Value {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
        Err(_) => serde_json::Value::Object(serde_json::Map::new()),
    }
}

fn save_json(path: &Path, data: &serde_json::Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let text = serde_json::to_string_pretty(data).unwrap_or_default();
    let _ = fs::write(path, text);
}

fn read_jsonl(path: &Path) -> Vec<serde_json::Value> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "{}" {
                return None;
            }
            serde_json::from_str(trimmed).ok()
        })
        .collect()
}

fn append_jsonl(path: &Path, obj: &serde_json::Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let line = serde_json::to_string(obj).unwrap_or_default();
    let mut content = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => String::new(),
    };
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&line);
    content.push('\n');
    let _ = fs::write(path, content);
}

fn rewrite_jsonl(path: &Path, records: &[serde_json::Value]) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut content = String::new();
    for record in records {
        content.push_str(&serde_json::to_string(record).unwrap_or_default());
        content.push('\n');
    }
    let _ = fs::write(path, content);
}

fn read_text(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn write_text(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, text);
}

fn simple_token_estimate(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    // 与 Python 的 len(text) // 2 对齐
    std::cmp::max(1, text.chars().count() / 2)
}

// ============================================================
// LLM 消息构建辅助
// ============================================================

fn sys_user_messages(system_prompt: &str, user_prompt: &str) -> Vec<llm::ChatMessage> {
    vec![
        llm::ChatMessage {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        },
        llm::ChatMessage {
            role: "user".to_string(),
            content: user_prompt.to_string(),
        },
    ]
}

// ============================================================
// 4.1 generate_chapter（移植 generate_chapter_local.py）
// ============================================================

const SYSTEM_PROMPT: &str = "你是中文长篇小说写手。

必须遵守：
- 只输出小说正文
- 不要解释
- 不要输出大纲
- 不要输出分析
- 不要写\u{201c}以下是\u{201d}
- 作者给的 chapter idea 是最高优先级
- 本章必须围绕 chapter idea 展开
- 不要扩写到下一章
- 本章内部必须连贯
- 不要重复同一事件
- 不要重复发放同一奖励
- 不要让同一个任务反复触发
- 年龄、天气、地点、金额、物品、任务状态等关键事实一旦确定，后文不能随便改变
- 不要与长期记忆冲突
- 如果长期记忆和 chapter idea 冲突，以 chapter idea 为准；潜在冲突由外部报告提示，不要写入正文
- 输出中文小说正文";

/// 把结构化数据转成适合 prompt 的文本（等价于 Python 的 dump_block）
fn dump_block_yaml(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::Null
        | serde_yaml::Value::Mapping(_)
        | serde_yaml::Value::Sequence(_) => {
            let text = serde_yaml::to_string(value).unwrap_or_default();
            let trimmed = text.trim();
            if trimmed.is_empty()
                || trimmed == "null"
                || trimmed == "---\n{}"
                || trimmed == "{}"
                || trimmed == "[]"
            {
                "（未提供）".to_string()
            } else {
                trimmed.to_string()
            }
        }
        _ => {
            let text = serde_yaml::to_string(value).unwrap_or_default();
            let trimmed = text.trim().to_string();
            if trimmed.is_empty() || trimmed == "null" {
                "（未提供）".to_string()
            } else {
                trimmed
            }
        }
    }
}

fn format_jsonl_records(records: &[serde_json::Value], empty_text: &str) -> String {
    if records.is_empty() {
        return empty_text.to_string();
    }
    records
        .iter()
        .map(|item| serde_json::to_string(item).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_style_references(records: &[serde_json::Value]) -> String {
    if records.is_empty() {
        return "（未提供）".to_string();
    }
    let parts: Vec<String> = records
        .iter()
        .filter_map(|item| {
            let sid = item
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("style");
            let text = item
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if text.is_empty() {
                None
            } else {
                Some(format!("[{}]\n{}", sid, text))
            }
        })
        .collect();
    if parts.is_empty() {
        "（未提供）".to_string()
    } else {
        parts.join("\n\n")
    }
}

struct MemorySections {
    sections: std::collections::HashMap<String, String>,
    conflict_sources: Vec<(String, String)>,
}

fn build_memory_sections(project_dir: &Path, use_context: bool) -> MemorySections {
    let skipped = "（--no-context 已启用，未读取长期记忆）".to_string();
    let mut sections = std::collections::HashMap::new();

    if !use_context {
        for key in [
            "Story Bible",
            "Characters",
            "Book Outline",
            "Recent Chapter Summaries",
            "Recent Events",
            "Timeline",
            "Active Foreshadowing",
            "Style References",
        ] {
            sections.insert(key.to_string(), skipped.clone());
        }
        return MemorySections {
            sections,
            conflict_sources: Vec::new(),
        };
    }

    let story_bible = load_yaml(&project_dir.join("memory").join("story_bible.yaml"));
    let characters = load_yaml(&project_dir.join("memory").join("characters.yaml"));
    let foreshadowing = load_yaml(&project_dir.join("memory").join("foreshadowing.yaml"));
    let book_outline = load_yaml(&project_dir.join("outlines").join("book_outline.yaml"));

    let mut chapter_summaries =
        read_jsonl(&project_dir.join("memory").join("chapter_summaries.jsonl"));
    let cs_len = chapter_summaries.len();
    if cs_len > 5 {
        chapter_summaries = chapter_summaries.split_off(cs_len - 5);
    }

    let mut events = read_jsonl(&project_dir.join("memory").join("events.jsonl"));
    let ev_len = events.len();
    if ev_len > 20 {
        events = events.split_off(ev_len - 20);
    }

    let mut timeline = read_jsonl(&project_dir.join("memory").join("timeline.jsonl"));
    let tl_len = timeline.len();
    if tl_len > 20 {
        timeline = timeline.split_off(tl_len - 20);
    }

    let style_bank_raw = read_jsonl(&project_dir.join("memory").join("style_bank.jsonl"));
    let style_bank: Vec<serde_json::Value> = style_bank_raw.into_iter().take(5).collect();

    sections.insert("Story Bible".to_string(), dump_block_yaml(&story_bible));
    sections.insert("Characters".to_string(), dump_block_yaml(&characters));
    sections.insert("Book Outline".to_string(), dump_block_yaml(&book_outline));
    sections.insert(
        "Recent Chapter Summaries".to_string(),
        format_jsonl_records(&chapter_summaries, "（暂无章节摘要）"),
    );
    sections.insert(
        "Recent Events".to_string(),
        format_jsonl_records(&events, "（暂无事件记录）"),
    );
    sections.insert(
        "Timeline".to_string(),
        format_jsonl_records(&timeline, "（暂无时间线记录）"),
    );
    sections.insert(
        "Active Foreshadowing".to_string(),
        dump_block_yaml(&foreshadowing),
    );
    sections.insert(
        "Style References".to_string(),
        format_style_references(&style_bank),
    );

    let conflict_keys = [
        "Story Bible",
        "Characters",
        "Book Outline",
        "Recent Chapter Summaries",
        "Recent Events",
        "Timeline",
        "Active Foreshadowing",
    ];
    let conflict_sources: Vec<(String, String)> = conflict_keys
        .iter()
        .filter_map(|k| sections.get(*k).map(|v| (k.to_string(), v.clone())))
        .collect();

    MemorySections {
        sections,
        conflict_sources,
    }
}

fn build_user_prompt(
    chapter_id: &str,
    idea: &str,
    target_words: u32,
    sections: &std::collections::HashMap<String, String>,
    form_block: &str,
) -> String {
    let get = |k: &str| sections.get(k).map(|s| s.as_str()).unwrap_or("（未提供）");
    format!(
        r#"# Chapter Generation Context

## Chapter ID
{chapter_id}

## Author Chapter Idea
{idea}

## Target Length
约 {target_words} 中文字

## Story Bible
{story_bible}

## Characters
{characters}

## Book Outline
{book_outline}

## Recent Chapter Summaries
{summaries}

## Recent Events
{events}

## Timeline
{timeline}

## Active Foreshadowing
{foreshadowing}

## Style References
{style}

{form_block}
## Hard Rules For This Chapter
- 只写本章正文
- 不要写章节分析
- 不要输出 markdown 标题
- 不要重复前文已发生事件
- 不要在本章内部重复同一个发现、奖励、任务、冲突或转折
- 结尾可以留钩子，但不要提前进入下一章主要剧情"#,
        chapter_id = chapter_id,
        idea = idea,
        target_words = target_words,
        story_bible = get("Story Bible"),
        characters = get("Characters"),
        book_outline = get("Book Outline"),
        summaries = get("Recent Chapter Summaries"),
        events = get("Recent Events"),
        timeline = get("Timeline"),
        foreshadowing = get("Active Foreshadowing"),
        style = get("Style References"),
        form_block = form_block,
    )
}

/// 把 POV 预设展开成英文指令；并组装「形式 / 叙事控制」英文区块。
/// 经验:形式(POV/规则)用英文表述更能贴合模型 CoT，内容(idea/记忆)保持中文。
fn build_form_block(pov: &str, directives: &[String]) -> String {
    let pov = pov.trim();
    let pov_line = if pov.is_empty() {
        "Follow the story's established/default point of view.".to_string()
    } else if let Some(rest) = pov.strip_prefix("objective") {
        let _ = rest;
        "Objective / observer POV: an invisible external observer (camera & microphone). \
Describe ONLY what is outwardly visible and audible. Do NOT state any character's thoughts, \
feelings, intentions, memories, or unstated knowledge — reveal interior states only through \
visible action, speech, and physical detail."
            .to_string()
    } else if let Some(c) = pov.strip_prefix("limited:").or_else(|| pov.strip_prefix("limited")) {
        let who = c.trim_start_matches(':').trim();
        if who.is_empty() {
            "Close third-person limited. Interiority is allowed ONLY for the single POV character; \
other characters are observed from outside.".to_string()
        } else {
            format!(
                "Close third-person limited on 「{who}」. Interiority is allowed ONLY for 「{who}」; \
all other characters are observed strictly from the outside.",
                who = who
            )
        }
    } else if let Some(c) = pov.strip_prefix("first:").or_else(|| pov.strip_prefix("first")) {
        let who = c.trim_start_matches(':').trim();
        if who.is_empty() {
            "First-person narration by the POV character.".to_string()
        } else {
            format!("First-person narration by 「{who}」.", who = who)
        }
    } else if pov.starts_with("omniscient") {
        "Omniscient narration (use sparingly; avoid head-hopping within a single scene)."
            .to_string()
    } else {
        // 自由描述,原样作为 POV 指令
        pov.to_string()
    };

    let directives_text = if directives.is_empty() {
        "（none specified — rely on the Story Bible's tone）".to_string()
    } else {
        directives
            .iter()
            .map(|d| format!("- {}", d.trim()))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        r#"## Form & Narrative Control (FORM — follow strictly; this overrides generic writing habits)

### Point of View
{pov_line}

### Narrative Directives
{directives_text}

### Formal Rules
- Honor the Point of View above without exception.
- Items flagged with `dramatize: true` (in foreshadowing / outline) MUST be rendered as a concrete in-scene beat experienced by the POV character (e.g. 「王二注意到……」), NOT summarized or delivered as authorial exposition.
- Resolve every action to an explicitly named character using the Characters list. The idea / outline / summaries may use loose pronouns; a parenthetical such as 「他(王二)」 is AUTHORITATIVE — the name in parentheses is the true referent. Never swap who performs an action with who receives it.

"#,
        pov_line = pov_line,
        directives_text = directives_text,
    )
}

/// 读取 story_bible.yaml 的书级叙事默认：narrative_defaults: { pov, directives: [...] }
fn read_narrative_defaults(project_dir: &Path) -> (String, Vec<String>) {
    let sb = load_yaml(&project_dir.join("memory").join("story_bible.yaml"));
    let nd = match sb.get("narrative_defaults") {
        Some(v) => v,
        None => return (String::new(), Vec::new()),
    };
    let pov = nd
        .get("pov")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let directives = nd
        .get("directives")
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    (pov, directives)
}

/// 从 idea 文本中提取关键词（移植 Python 的 idea_terms）
fn idea_terms(idea: &str) -> std::collections::HashSet<String> {
    let mut terms = std::collections::HashSet::new();
    let re = regex::Regex::new(r"[一-鿿A-Za-z0-9_]{2,}").unwrap();
    for mat in re.find_iter(idea) {
        let chunk = mat.as_str();
        if chunk.chars().count() <= 6 {
            terms.insert(chunk.to_string());
        }
        let chars: Vec<char> = chunk.chars().collect();
        for i in 0..chars.len().saturating_sub(1) {
            let bigram: String = chars[i..=i + 1].iter().collect();
            terms.insert(bigram);
        }
    }
    terms
}

/// 轻量字面冲突提示（移植 Python 的 detect_potential_conflicts）
fn detect_potential_conflicts(idea: &str, sources: &[(String, String)]) -> Vec<String> {
    let terms = idea_terms(idea);
    if terms.is_empty() {
        return Vec::new();
    }

    let markers = [
        "年龄", "岁", "天气", "地点", "位置", "金额", "价格", "物品", "身份",
        "状态", "死亡", "失踪", "禁止", "不能", "不要", "秘密", "已", "已经",
        "任务", "奖励", "发现",
    ];

    let mut warnings = Vec::new();
    for (label, text) in sources {
        for raw_line in text.lines() {
            let line = raw_line.trim();
            if line.chars().count() < 6 || !markers.iter().any(|m| line.contains(m)) {
                continue;
            }
            let overlap: Vec<&String> = terms
                .iter()
                .filter(|t| t.chars().count() >= 2 && line.contains(t.as_str()))
                .collect();
            if overlap.len() >= 2 {
                let truncated: String = line.chars().take(180).collect();
                warnings.push(format!("- [{}] {}", label, truncated));
                if warnings.len() >= 8 {
                    return warnings;
                }
            }
        }
    }
    warnings
}

/// 清理模型输出（移植 Python 的 clean_output）
fn clean_output(raw: &str) -> String {
    let mut text = raw.trim().to_string();
    let prefixes = [
        "以下是小说正文：",
        "以下是正文：",
        "下面是小说正文：",
        "下面是正文：",
        "好的，",
        "好的。",
    ];
    for prefix in &prefixes {
        if text.starts_with(prefix) {
            text = text[prefix.len()..].trim().to_string();
            break;
        }
    }
    if text.starts_with("```") {
        if let Some(first_nl) = text.find('\n') {
            text = text[first_nl + 1..].to_string();
        }
        if text.trim_end().ends_with("```") {
            let trimmed = text.trim_end();
            text = trimmed[..trimmed.len() - 3].trim_end().to_string();
        }
    }
    text
}

fn write_generation_report(
    project_dir: &Path,
    chapter_id: &str,
    idea: &str,
    target_words: u32,
    used_context: bool,
    context_tokens: usize,
    output_text: &str,
    raw_response: &str,
    _user_prompt: &str,
    conflict_warnings: &[String],
    config: &RunConfig,
) {
    let warning_text = if conflict_warnings.is_empty() {
        "未发现明显字面冲突；仍建议作者人工审阅长期记忆与本章 idea。".to_string()
    } else {
        conflict_warnings.join("\n")
    };

    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");

    let report = format!(
        r#"# Chapter Generation Report: {chapter_id}

## Chapter ID

{chapter_id}

## Idea

{idea}

## Target Words

{target_words}

## Used Context

{used_context}

## Context Token Estimate

{context_tokens}

## Potential Context Conflicts

{warning_text}

## Output Length

{output_len} characters

## Model Info

- model: {model}
- base_url: {base_url}

## Generation Time

{now}

## Raw Response

```text
{raw_response}
```"#,
        chapter_id = chapter_id,
        idea = idea,
        target_words = target_words,
        used_context = used_context,
        context_tokens = context_tokens,
        warning_text = warning_text,
        output_len = output_text.chars().count(),
        model = config.model_name,
        base_url = config.llm_base_url,
        now = now,
        raw_response = raw_response,
    );

    let report_path = project_dir
        .join("outputs")
        .join("reports")
        .join(format!("{}_chapter_generation_report.md", chapter_id));
    write_text(&report_path, &report);
}

fn do_generate_chapter(
    app: &tauri::AppHandle,
    chapter_id: &str,
    idea: &str,
    target_words: u32,
    use_context: bool,
    overwrite: bool,
    pov: &str,
    narrative: &str,
) -> CommandResult {
    let config = match build_run_config(app) {
        Ok(c) => c,
        Err(e) => return CommandResult { success: false, log: e },
    };
    let settings = load_settings(app);

    let chapter_path = config
        .project_dir
        .join("chapters")
        .join(chapter_id)
        .join("chapter.md");

    if chapter_path.exists() && !overwrite {
        return CommandResult {
            success: false,
            log: format!("{} 已存在。请勾选「覆盖」以重新生成。", chapter_path.display()),
        };
    }

    // 叙事控制：章级输入优先，回退到 story_bible.narrative_defaults（书级默认）
    let (default_pov, default_directives) = read_narrative_defaults(&config.project_dir);
    let effective_pov = if pov.trim().is_empty() { default_pov } else { pov.trim().to_string() };
    let effective_directives: Vec<String> = if narrative.trim().is_empty() {
        default_directives
    } else {
        narrative
            .split(|c| c == ',' || c == '，' || c == '\n')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };
    let form_block = build_form_block(&effective_pov, &effective_directives);

    let mem = build_memory_sections(&config.project_dir, use_context);
    let user_prompt = build_user_prompt(chapter_id, idea, target_words, &mem.sections, &form_block);
    let context_tokens = simple_token_estimate(&user_prompt);
    let conflict_warnings = if use_context {
        detect_potential_conflicts(idea, &mem.conflict_sources)
    } else {
        Vec::new()
    };

    let mut log = String::new();
    log.push_str(&format!("[信息] 章节: {}\n", chapter_id));
    log.push_str(&format!("[信息] 目标长度: 约 {} 中文字\n", target_words));
    log.push_str(&format!("[信息] 使用长期记忆: {}\n", use_context));
    log.push_str(&format!("[信息] prompt token 估算: ~{}\n", context_tokens));

    let messages = sys_user_messages(SYSTEM_PROMPT, &apply_no_think(user_prompt.clone(), &settings));

    let temperature = settings.temperature.unwrap_or(0.8);
    let top_p = settings.top_p.unwrap_or(0.9);

    log.push_str("[信息] 正在调用模型生成完整章节...\n");

    let raw_response = match llm::call_llm(
        &config.llm_base_url,
        &config.llm_api_key,
        &config.model_name,
        messages,
        temperature,
        top_p,
        config.max_output_tokens,
    ) {
        Ok(r) => r,
        Err(e) => {
            return CommandResult {
                success: false,
                log: format!("{}[错误] 模型调用失败: {}", log, e),
            };
        }
    };

    let output_text = clean_output(&raw_response);

    write_text(&chapter_path, &output_text);
    log.push_str(&format!("[完成] 章节正文已保存: {}\n", chapter_path.display()));

    write_generation_report(
        &config.project_dir,
        chapter_id,
        idea,
        target_words,
        use_context,
        context_tokens,
        &output_text,
        &raw_response,
        &user_prompt,
        &conflict_warnings,
        &config,
    );
    log.push_str("[完成] 生成报告已保存\n");

    CommandResult {
        success: true,
        log: log.trim().to_string(),
    }
}

// ============================================================
// 4.2 check_consistency（移植 check_consistency.py）
// ============================================================

const CONSISTENCY_CHECK_PROMPT: &str = r#"你是一位经验丰富的小说编辑，专门负责长篇小说的连续性审查。

请仔细阅读以下完整章节正文和相关记忆档案，找出所有一致性问题。

## 检查清单

1. **人物语气一致性**: 角色的说话方式是否与角色档案中描述的一致？
2. **信息超前**: 角色是否知道了他们不应该知道的信息？（参考 knows 和 secrets 字段）
3. **世界观冲突**: 正文中是否违反了 Story Bible 中的世界观规则？
4. **时间线矛盾**: 事件的时间顺序是否有矛盾？
5. **伏笔问题**: 伏笔是否遗漏、误回收、过早揭示或互相冲突？
6. **重复事件**: 是否重复前文已发生的同一事件？
7. **重复任务/奖励/发现**: 是否重复触发同一任务，重复发放同一奖励，或重复发现同一信息？
8. **关键事实漂移**: 年龄、天气、地点、金额、物品、身份、任务状态等是否前后改变？
9. **AI 腔检查**: 是否有明显的 AI 生成痕迹？（总结腔、解释腔、列举句式等）
10. **Chapter Idea 执行**: 是否违背或偏离作者给出的 chapter idea？

## 章节正文

{chapter_text}

## 作者 Chapter Idea

{chapter_idea}

## 角色档案

{characters_text}

## 世界观规则

{world_rules_text}

## 活跃伏笔

{foreshadowing_text}

## 最近事件

{events_text}

## 最近时间线

{timeline_text}

## 输出要求

请输出一份 Markdown 格式的审查报告。格式如下：

# 一致性审查报告: {chapter_id}

## 总体评价
（一段话概括质量）

## 发现的问题

### 严重问题
- **问题**: （描述）
  - **位置**: （大约在哪一段）
  - **原因**: （为什么是问题）
  - **建议**: （如何修复）

### 中等问题
- ...

### 轻微问题/建议
- ...

## 亮点
（如果有的话，指出写得好的地方）

如果没有任何问题，请如实说明。"#;

/// 从生成报告中解析 chapter idea
fn load_chapter_idea(project_dir: &Path, chapter_id: &str) -> String {
    let report_path = project_dir
        .join("outputs")
        .join("reports")
        .join(format!("{}_chapter_generation_report.md", chapter_id));
    let text = read_text(&report_path);
    if text.is_empty() {
        return "（未找到生成报告中的 chapter idea；如需检查此项，请参考作者原始输入。）".to_string();
    }
    let re = regex::Regex::new(r"(?s)## Idea\s+(.*?)\s+## Target Words").unwrap();
    match re.captures(&text) {
        Some(caps) => caps.get(1).map(|m| m.as_str().trim().to_string()).unwrap_or_else(|| {
            "（未能从生成报告解析 chapter idea；如需检查此项，请参考作者原始输入。）".to_string()
        }),
        None => "（未能从生成报告解析 chapter idea；如需检查此项，请参考作者原始输入。）".to_string(),
    }
}

fn do_check_consistency(app: &tauri::AppHandle, chapter_id: &str) -> CommandResult {
    let config = match build_run_config(app) {
        Ok(c) => c,
        Err(e) => return CommandResult { success: false, log: e },
    };
    let settings = load_settings(app);

    let chapter_path = config
        .project_dir
        .join("chapters")
        .join(chapter_id)
        .join("chapter.md");

    if !chapter_path.exists() {
        return CommandResult {
            success: false,
            log: format!(
                "[错误] 未找到 {}\n[提示] 请先生成该章节。",
                chapter_path.display()
            ),
        };
    }

    let chapter_text = read_text(&chapter_path);
    if chapter_text.trim().is_empty() {
        return CommandResult {
            success: false,
            log: format!("[错误] {} 是空文件", chapter_path.display()),
        };
    }

    let chapter_idea = load_chapter_idea(&config.project_dir, chapter_id);

    let characters = load_yaml(&config.project_dir.join("memory").join("characters.yaml"));
    let story_bible = load_yaml(&config.project_dir.join("memory").join("story_bible.yaml"));
    let foreshadowing = load_yaml(&config.project_dir.join("memory").join("foreshadowing.yaml"));
    let events = read_jsonl(&config.project_dir.join("memory").join("events.jsonl"));
    let timeline = read_jsonl(&config.project_dir.join("memory").join("timeline.jsonl"));

    let recent_events: Vec<&serde_json::Value> =
        events.iter().rev().take(20).rev().collect();
    let recent_timeline: Vec<&serde_json::Value> =
        timeline.iter().rev().take(20).rev().collect();

    let characters_text = serde_json::to_string_pretty(&characters).unwrap_or_default();

    // 世界观规则文本
    let world_rules = story_bible
        .get("world_rules")
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str())
                .map(|s| format!("- {}", s))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    let writing_rules = story_bible
        .get("writing_rules")
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str())
                .map(|s| format!("- {}", s))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    let forbidden = story_bible
        .get("forbidden_patterns")
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str())
                .map(|s| format!("- {}", s))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    let world_rules_text = format!(
        "世界观规则:\n{}\n\n写作规则:\n{}\n\n禁止模式:\n{}",
        world_rules, writing_rules, forbidden
    );

    let foreshadowing_text =
        serde_json::to_string_pretty(&foreshadowing).unwrap_or_default();

    let events_text = if recent_events.is_empty() {
        "（暂无事件记录）".to_string()
    } else {
        recent_events
            .iter()
            .map(|e| {
                let eid = e.get("event_id").and_then(|v| v.as_str()).unwrap_or("?");
                let desc = e.get("description").and_then(|v| v.as_str()).unwrap_or("");
                format!("- [{}] {}", eid, desc)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let timeline_text = if recent_timeline.is_empty() {
        "（暂无时间线记录）".to_string()
    } else {
        recent_timeline
            .iter()
            .map(|t| {
                let eid = t.get("event_id").and_then(|v| v.as_str()).unwrap_or("?");
                let time = t.get("time").and_then(|v| v.as_str()).unwrap_or("");
                let desc = t.get("description").and_then(|v| v.as_str()).unwrap_or("");
                format!("- [{}] {}: {}", eid, time, desc)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let prompt = CONSISTENCY_CHECK_PROMPT
        .replace("{chapter_id}", chapter_id)
        .replace("{chapter_text}", &chapter_text)
        .replace("{chapter_idea}", &chapter_idea)
        .replace("{characters_text}", &characters_text)
        .replace("{world_rules_text}", &world_rules_text)
        .replace("{foreshadowing_text}", &foreshadowing_text)
        .replace("{events_text}", &events_text)
        .replace("{timeline_text}", &timeline_text);

    let messages = sys_user_messages(
        "你是一位专业的小说编辑，专注于连续性审查。请用中文输出报告。",
        &apply_no_think(prompt.clone(), &settings),
    );

    let top_p = settings.top_p.unwrap_or(0.9);

    let mut log = format!("[信息] 正在对 {} 进行一致性审查...\n", chapter_id);

    let report = match llm::call_llm(
        &config.llm_base_url,
        &config.llm_api_key,
        &config.model_name,
        messages,
        0.4,
        top_p,
        config.max_output_tokens,
    ) {
        Ok(r) => r,
        Err(e) => {
            return CommandResult {
                success: false,
                log: format!("{}[错误] 模型调用失败: {}", log, e),
            };
        }
    };

    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let config_yaml = load_config_yaml(&config.project_dir);
    let model_name_cfg = config_string(&config_yaml, "model_name", "未知");
    let header = format!(
        "<!-- 一致性审查报告 -->\n<!-- 生成时间: {} -->\n<!-- 模型: {} -->\n\n",
        now, model_name_cfg
    );

    let report_path = config
        .project_dir
        .join("outputs")
        .join("reports")
        .join(format!("{}_consistency_report.md", chapter_id));
    write_text(&report_path, &format!("{}{}", header, report));

    log.push_str(&format!(
        "[完成] 一致性审查报告已保存: {}\n",
        report_path.display()
    ));

    CommandResult {
        success: true,
        log: log.trim().to_string(),
    }
}

// ============================================================
// 4.3 update_memory（移植 update_memory_after_chapter.py）
// ============================================================

const EXTRACTION_PROMPT_TEMPLATE: &str = r#"你是一位专业的小说编辑助手。请阅读以下完整章节正文，然后以严格的JSON格式输出结构化的长期记忆更新信息。

## 章节正文

{chapter_text}

## 现有角色档案

{characters_text}

## 现有伏笔

{foreshadowing_text}

## 输出要求

请输出一个严格的JSON对象，不要包含任何其他文字、解释或markdown标记。JSON schema 如下：

{
  "chapter_id": "{chapter_id}",
  "chapter_summary": "用2-3句话概括本章发生了什么",
  "events": [
    {
      "event_id": "{chapter_id}_evt001",
      "chapter_id": "{chapter_id}",
      "type": "chapter_event/conflict/revelation/decision/travel/battle/task/reward/discovery",
      "characters": ["角色名"],
      "description": "事件描述",
      "location": "地点",
      "time_order": "相对时间描述",
      "importance": 1到10的整数
    }
  ],
  "character_updates": [
    {
      "name": "角色名",
      "field": "current_status/knows/secrets/relationships/constraints",
      "old_value": "旧值（如果知道的话）",
      "new_value": "新值",
      "reason": "变化原因"
    }
  ],
  "foreshadowing_updates": [
    {
      "id": "伏笔ID",
      "action": "add/resolve/update",
      "description": "描述",
      "related_characters": ["角色名"],
      "planned_resolution": "计划回收方式",
      "status": "active/resolved"
    }
  ],
  "relationship_updates": [
    {
      "source": "角色A",
      "target": "角色B",
      "relation": "关系描述",
      "change": "变化说明"
    }
  ],
  "continuity_risks": [
    {
      "risk": "风险描述",
      "severity": "low/medium/high",
      "suggestion": "建议"
    }
  ]
}"#;

/// 从模型输出中提取 JSON（移植 Python 的 parse_extraction_json）
fn parse_extraction_json(raw_output: &str) -> Result<serde_json::Value, String> {
    let mut text = raw_output.trim().to_string();

    // 尝试从 ```json ... ``` 中提取
    let re_codeblock = regex::Regex::new(r"(?s)```(?:json)?\s*\n?(.*?)\n?```").unwrap();
    if let Some(caps) = re_codeblock.captures(&text) {
        if let Some(m) = caps.get(1) {
            text = m.as_str().trim().to_string();
        }
    }

    // 如果不是以 { 或 [ 开头，截取第一个 { 到最后一个 }
    if !text.is_empty() && !text.starts_with('{') && !text.starts_with('[') {
        if let Some(start) = text.find('{') {
            text = text[start..].to_string();
        }
    }
    if let Some(last) = text.rfind('}') {
        text = text[..=last].to_string();
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
        return Ok(value);
    }

    // 容错：模型输出被 token 上限截断时，抢救已完整的部分（截到最后一个完整元素并补全括号）
    if let Some(value) = repair_truncated_json(&text) {
        return Ok(value);
    }

    let preview: String = raw_output.chars().take(500).collect();
    match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(value) => Ok(value),
        Err(e) => Err(format!("JSON解析失败: {}\n原始输出前500字:\n{}", e, preview)),
    }
}

/// 尝试修复被截断的 JSON：截到最后一个完整元素（最后一个 } / ] 或逗号之前），
/// 再根据未闭合的括号补全。仅按字节扫描 ASCII 结构字符，UTF-8 续字节不会被误判。
fn repair_truncated_json(s: &str) -> Option<serde_json::Value> {
    let bytes = s.as_bytes();
    let mut in_str = false;
    let mut esc = false;
    let mut best: usize = 0; // 安全截断点（独占）
    for (i, &b) in bytes.iter().enumerate() {
        if in_str {
            if esc {
                esc = false;
            } else if b == b'\\' {
                esc = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'}' | b']' => best = i + 1, // 完整容器闭合后
            b',' => best = i,            // 逗号前（丢弃逗号及其后的残缺元素）
            _ => {}
        }
    }
    if best == 0 {
        return None;
    }

    let mut prefix = s[..best].to_string();

    // 重新计算 prefix 的未闭合括号栈
    let mut stack: Vec<char> = Vec::new();
    let mut in_s = false;
    let mut e = false;
    for &b in prefix.as_bytes() {
        if in_s {
            if e {
                e = false;
            } else if b == b'\\' {
                e = true;
            } else if b == b'"' {
                in_s = false;
            }
            continue;
        }
        match b {
            b'"' => in_s = true,
            b'{' => stack.push('}'),
            b'[' => stack.push(']'),
            b'}' | b']' => {
                stack.pop();
            }
            _ => {}
        }
    }
    while let Some(close) = stack.pop() {
        prefix.push(close);
    }

    serde_json::from_str(&prefix).ok()
}

/// 应用记忆更新（移植 Python 的 apply_memory_updates）
fn apply_memory_updates(data: &serde_json::Value, chapter_id: &str, project_dir: &Path) {
    // 1. 追加章节摘要
    let summary = data
        .get("chapter_summary")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let mut summary_obj = serde_json::Map::new();
    summary_obj.insert(
        "chapter_id".to_string(),
        serde_json::Value::String(chapter_id.to_string()),
    );
    summary_obj.insert(
        "chapter_summary".to_string(),
        serde_json::Value::String(summary.to_string()),
    );
    append_jsonl(
        &project_dir.join("memory").join("chapter_summaries.jsonl"),
        &serde_json::Value::Object(summary_obj),
    );

    // 2. 追加事件
    let events = data.get("events").and_then(|v| v.as_array());
    if let Some(events) = events {
        for evt in events {
            let mut evt = evt.clone();
            if let Some(obj) = evt.as_object_mut() {
                obj.entry("chapter_id".to_string()).or_insert_with(|| {
                    serde_json::Value::String(chapter_id.to_string())
                });
            }
            append_jsonl(
                &project_dir.join("memory").join("events.jsonl"),
                &evt,
            );
        }
    }

    // 3. 追加 timeline
    if let Some(events) = events {
        for evt in events {
            let event_id = evt
                .get("event_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let time_order = evt
                .get("time_order")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let description = evt
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let characters = evt
                .get("characters")
                .cloned()
                .unwrap_or(serde_json::Value::Array(vec![]));
            let importance = evt
                .get("importance")
                .and_then(|v| v.as_u64())
                .unwrap_or(5);

            let mut tl_obj = serde_json::Map::new();
            tl_obj.insert(
                "event_id".to_string(),
                serde_json::Value::String(event_id.to_string()),
            );
            tl_obj.insert(
                "chapter_id".to_string(),
                serde_json::Value::String(chapter_id.to_string()),
            );
            tl_obj.insert(
                "time".to_string(),
                serde_json::Value::String(time_order.to_string()),
            );
            tl_obj.insert(
                "description".to_string(),
                serde_json::Value::String(description.to_string()),
            );
            tl_obj.insert("characters".to_string(), characters);
            tl_obj.insert(
                "importance".to_string(),
                serde_json::Value::Number(importance.into()),
            );
            append_jsonl(
                &project_dir.join("memory").join("timeline.jsonl"),
                &serde_json::Value::Object(tl_obj),
            );
        }
    }

    // 4. 更新角色档案
    if let Some(updates) = data.get("character_updates").and_then(|v| v.as_array()) {
        update_characters(updates, project_dir);
    }

    // 5. 更新伏笔
    if let Some(updates) = data.get("foreshadowing_updates").and_then(|v| v.as_array()) {
        update_foreshadowing(updates, project_dir);
    }

    // 6. 更新关系图
    if let Some(updates) = data.get("relationship_updates").and_then(|v| v.as_array()) {
        update_relationships(updates, project_dir);
    }
}

fn update_characters(updates: &[serde_json::Value], project_dir: &Path) {
    let chars_path = project_dir.join("memory").join("characters.yaml");
    let mut chars_data = load_yaml(&chars_path);

    // 构建 name -> index 映射
    let char_map: std::collections::HashMap<String, usize> = {
        let empty = vec![];
        let characters = chars_data
            .get("characters")
            .and_then(|v| v.as_sequence())
            .unwrap_or(&empty);
        characters
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                c.get("name")
                    .and_then(|v| v.as_str())
                    .map(|n| (n.to_string(), i))
            })
            .collect()
    };

    let characters = chars_data
        .get_mut("characters")
        .and_then(|v| v.as_sequence_mut());

    let characters = match characters {
        Some(c) => c,
        None => return,
    };

    for update in updates {
        let name = update
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let field = update
            .get("field")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let new_value = update
            .get("new_value")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let idx = match char_map.get(name) {
            Some(i) => *i,
            None => {
                eprintln!("[警告] 角色 '{}' 不存在，跳过", name);
                continue;
            }
        };

        let char = &mut characters[idx];

        if field == "current_status" {
            if let serde_yaml::Value::Mapping(ref mut map) = char {
                map.insert(
                    serde_yaml::Value::String("current_status".to_string()),
                    serde_yaml::Value::String(new_value.to_string()),
                );
            }
        } else if field == "knows"
            || field == "secrets"
            || field == "constraints"
            || field == "relationships"
        {
            // 获取当前列表
            if let Some(list) = char.get(field).and_then(|v| v.as_sequence()) {
                // 检查是否已存在
                let exists = list
                    .iter()
                    .any(|v| v.as_str() == Some(new_value));
                if !exists {
                    if let Some(list) = char.get_mut(field).and_then(|v| v.as_sequence_mut()) {
                        list.push(serde_yaml::Value::String(new_value.to_string()));
                    }
                }
            } else {
                // 字段不存在或是 None，创建新列表
                if let Some(char_mapping) = char.as_mapping_mut() {
                    let mut new_list = serde_yaml::Sequence::new();
                    new_list.push(serde_yaml::Value::String(new_value.to_string()));
                    char_mapping.insert(
                        serde_yaml::Value::String(field.to_string()),
                        serde_yaml::Value::Sequence(new_list),
                    );
                }
            }
        }
    }

    save_yaml(&chars_path, &chars_data);
}

fn update_foreshadowing(updates: &[serde_json::Value], project_dir: &Path) {
    let fs_path = project_dir.join("memory").join("foreshadowing.yaml");
    let mut fs_data = load_yaml(&fs_path);

    // 确保 active 和 resolved 存在
    if fs_data.get("active").is_none() {
        if let Some(map) = fs_data.as_mapping_mut() {
            map.insert(
                serde_yaml::Value::String("active".to_string()),
                serde_yaml::Value::Sequence(serde_yaml::Sequence::new()),
            );
        }
    }
    if fs_data.get("resolved").is_none() {
        if let Some(map) = fs_data.as_mapping_mut() {
            map.insert(
                serde_yaml::Value::String("resolved".to_string()),
                serde_yaml::Value::Sequence(serde_yaml::Sequence::new()),
            );
        }
    }

    // 构建 active id -> index 映射
    let active_map: std::collections::HashMap<String, usize> = {
        let empty_seq = serde_yaml::Sequence::new();
        let active = fs_data
            .get("active")
            .and_then(|v| v.as_sequence())
            .unwrap_or(&empty_seq);
        active
            .iter()
            .enumerate()
            .filter_map(|(i, fs)| {
                fs.get("id")
                    .and_then(|v| v.as_str())
                    .map(|id| (id.to_string(), i))
            })
            .collect()
    };

    for update in updates {
        let action = update
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let fs_id = update
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if action == "add" {
            let mut new_entry = serde_yaml::Mapping::new();
            new_entry.insert(
                serde_yaml::Value::String("id".to_string()),
                serde_yaml::Value::String(fs_id.to_string()),
            );
            let introduced = update
                .get("introduced_in")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            new_entry.insert(
                serde_yaml::Value::String("introduced_in".to_string()),
                serde_yaml::Value::String(introduced.to_string()),
            );
            let desc = update
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            new_entry.insert(
                serde_yaml::Value::String("description".to_string()),
                serde_yaml::Value::String(desc.to_string()),
            );
            let related: serde_yaml::Value = serde_json::from_str(
                &serde_json::to_string(
                    &update
                        .get("related_characters")
                        .cloned()
                        .unwrap_or(serde_json::Value::Array(vec![])),
                )
                .unwrap_or_default(),
            )
            .unwrap_or(serde_yaml::Value::Sequence(serde_yaml::Sequence::new()));
            new_entry.insert(
                serde_yaml::Value::String("related_characters".to_string()),
                related,
            );
            let planned = update
                .get("planned_resolution")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            new_entry.insert(
                serde_yaml::Value::String("planned_resolution".to_string()),
                serde_yaml::Value::String(planned.to_string()),
            );
            new_entry.insert(
                serde_yaml::Value::String("status".to_string()),
                serde_yaml::Value::String("active".to_string()),
            );
            if let Some(active) = fs_data
                .get_mut("active")
                .and_then(|v| v.as_sequence_mut())
            {
                active.push(serde_yaml::Value::Mapping(new_entry));
            }
        } else if action == "resolve" {
            if let Some(idx) = active_map.get(fs_id) {
                let active = fs_data
                    .get_mut("active")
                    .and_then(|v| v.as_sequence_mut());
                if let Some(active) = active {
                    if *idx < active.len() {
                        let mut entry = active.remove(*idx);
                        // 设置 status = resolved
                        if let Some(map) = entry.as_mapping_mut() {
                            map.insert(
                                serde_yaml::Value::String("status".to_string()),
                                serde_yaml::Value::String("resolved".to_string()),
                            );
                        }
                        if let Some(resolved) = fs_data
                            .get_mut("resolved")
                            .and_then(|v| v.as_sequence_mut())
                        {
                            resolved.push(entry);
                        }
                    }
                }
            }
        } else if action == "update" {
            if let Some(idx) = active_map.get(fs_id) {
                let active = fs_data
                    .get_mut("active")
                    .and_then(|v| v.as_sequence_mut());
                if let Some(active) = active {
                    if *idx < active.len() {
                        let entry = &mut active[*idx];
                        if let Some(desc) = update.get("description").and_then(|v| v.as_str()) {
                            if let Some(map) = entry.as_mapping_mut() {
                                map.insert(
                                    serde_yaml::Value::String("description".to_string()),
                                    serde_yaml::Value::String(desc.to_string()),
                                );
                            }
                        }
                        if let Some(planned) =
                            update.get("planned_resolution").and_then(|v| v.as_str())
                        {
                            if let Some(map) = entry.as_mapping_mut() {
                                map.insert(
                                    serde_yaml::Value::String("planned_resolution".to_string()),
                                    serde_yaml::Value::String(planned.to_string()),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    save_yaml(&fs_path, &fs_data);
}

fn update_relationships(updates: &[serde_json::Value], project_dir: &Path) {
    let rel_path = project_dir.join("memory").join("relationships.json");
    let mut rel_data = load_json(&rel_path);

    // 确保 nodes 和 edges 存在
    if rel_data.get("nodes").is_none() {
        if let Some(obj) = rel_data.as_object_mut() {
            obj.insert("nodes".to_string(), serde_json::Value::Array(vec![]));
        }
    }
    if rel_data.get("edges").is_none() {
        if let Some(obj) = rel_data.as_object_mut() {
            obj.insert("edges".to_string(), serde_json::Value::Array(vec![]));
        }
    }

    // 构建 node_ids set
    let mut node_ids: std::collections::HashSet<String> = {
        let empty = vec![];
        let nodes = rel_data
            .get("nodes")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty);
        nodes
            .iter()
            .filter_map(|n| n.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .collect()
    };

    // 构建 edge_keys set
    let mut edge_keys: std::collections::HashSet<(String, String)> = {
        let empty = vec![];
        let edges = rel_data
            .get("edges")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty);
        edges
            .iter()
            .filter_map(|e| {
                let source = e.get("source").and_then(|v| v.as_str())?;
                let target = e.get("target").and_then(|v| v.as_str())?;
                Some((source.to_string(), target.to_string()))
            })
            .collect()
    };

    for update in updates {
        let source = update
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let target = update
            .get("target")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let relation = update
            .get("relation")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // 缺的节点补建
        if !node_ids.contains(source) {
            let mut node = serde_json::Map::new();
            node.insert("id".to_string(), serde_json::Value::String(source.to_string()));
            node.insert("type".to_string(), serde_json::Value::String("character".to_string()));
            node.insert("role".to_string(), serde_json::Value::String("".to_string()));
            if let Some(nodes) = rel_data
                .get_mut("nodes")
                .and_then(|v| v.as_array_mut())
            {
                nodes.push(serde_json::Value::Object(node));
            }
            node_ids.insert(source.to_string());
        }
        if !node_ids.contains(target) {
            let mut node = serde_json::Map::new();
            node.insert("id".to_string(), serde_json::Value::String(target.to_string()));
            node.insert("type".to_string(), serde_json::Value::String("character".to_string()));
            node.insert("role".to_string(), serde_json::Value::String("".to_string()));
            if let Some(nodes) = rel_data
                .get_mut("nodes")
                .and_then(|v| v.as_array_mut())
            {
                nodes.push(serde_json::Value::Object(node));
            }
            node_ids.insert(target.to_string());
        }

        let edge_key = (source.to_string(), target.to_string());
        if edge_keys.contains(&edge_key) {
            // 更新已有边
            if let Some(edges) = rel_data
                .get_mut("edges")
                .and_then(|v| v.as_array_mut())
            {
                for e in edges.iter_mut() {
                    let s = e.get("source").and_then(|v| v.as_str()).unwrap_or("");
                    let t = e.get("target").and_then(|v| v.as_str()).unwrap_or("");
                    if s == source && t == target {
                        if let Some(obj) = e.as_object_mut() {
                            obj.insert(
                                "relation".to_string(),
                                serde_json::Value::String(relation.to_string()),
                            );
                            if let Some(change) =
                                update.get("change").and_then(|v| v.as_str())
                            {
                                obj.insert(
                                    "change".to_string(),
                                    serde_json::Value::String(change.to_string()),
                                );
                            }
                        }
                        break;
                    }
                }
            }
        } else {
            // 新增边
            let mut edge = serde_json::Map::new();
            edge.insert(
                "source".to_string(),
                serde_json::Value::String(source.to_string()),
            );
            edge.insert(
                "target".to_string(),
                serde_json::Value::String(target.to_string()),
            );
            edge.insert(
                "relation".to_string(),
                serde_json::Value::String(relation.to_string()),
            );
            let change = update
                .get("change")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            edge.insert(
                "change".to_string(),
                serde_json::Value::String(change.to_string()),
            );
            if let Some(edges) = rel_data
                .get_mut("edges")
                .and_then(|v| v.as_array_mut())
            {
                edges.push(serde_json::Value::Object(edge));
            }
            edge_keys.insert(edge_key);
        }
    }

    save_json(&rel_path, &rel_data);
}

fn generate_memory_report(
    data: &serde_json::Value,
    chapter_id: &str,
    raw_output: &str,
    project_dir: &Path,
) {
    let summary = data
        .get("chapter_summary")
        .and_then(|v| v.as_str())
        .unwrap_or("无");
    let events = data
        .get("events")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let char_updates = data
        .get("character_updates")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let fs_updates = data
        .get("foreshadowing_updates")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let rel_updates = data
        .get("relationship_updates")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let risks = data
        .get("continuity_risks")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut lines = Vec::new();

    lines.push(format!("# 记忆更新报告: {}\n", chapter_id));
    lines.push(format!("## 章节摘要\n\n{}\n", summary));
    lines.push(format!("## 新增事件 ({})\n", events.len()));

    for evt in &events {
        let eid = evt.get("event_id").and_then(|v| v.as_str()).unwrap_or("?");
        let desc = evt
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        lines.push(format!("- **[{}]** {}", eid, desc));
        let etype = evt.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let chars = evt
            .get("characters")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        let loc = evt
            .get("location")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        lines.push(format!(
            "  - 类型: {} | 角色: {} | 地点: {}\n",
            etype, chars, loc
        ));
    }

    lines.push(format!("## 角色更新 ({})\n", char_updates.len()));
    for cu in &char_updates {
        let name = cu.get("name").and_then(|v| v.as_str()).unwrap_or("?");
        let field = cu.get("field").and_then(|v| v.as_str()).unwrap_or("?");
        let new_val = cu
            .get("new_value")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        lines.push(format!("- **{}.{}**: {}", name, field, new_val));
        let reason = cu.get("reason").and_then(|v| v.as_str()).unwrap_or("");
        lines.push(format!("  - 原因: {}\n", reason));
    }

    lines.push(format!("## 伏笔更新 ({})\n", fs_updates.len()));
    for fu in &fs_updates {
        let fid = fu.get("id").and_then(|v| v.as_str()).unwrap_or("?");
        let fact = fu.get("action").and_then(|v| v.as_str()).unwrap_or("");
        let desc = fu
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        lines.push(format!("- **[{}]** {}: {}\n", fid, fact, desc));
    }

    lines.push(format!("## 关系更新 ({})\n", rel_updates.len()));
    for ru in &rel_updates {
        let src = ru.get("source").and_then(|v| v.as_str()).unwrap_or("?");
        let rel = ru.get("relation").and_then(|v| v.as_str()).unwrap_or("?");
        let tgt = ru.get("target").and_then(|v| v.as_str()).unwrap_or("?");
        lines.push(format!("- {} --[{}]--> {}\n", src, rel, tgt));
    }

    lines.push(format!("## 连续性风险 ({})\n", risks.len()));
    for risk in &risks {
        let severity = risk.get("severity").and_then(|v| v.as_str()).unwrap_or("low");
        let severity_cn = match severity {
            "high" => "高",
            "medium" => "中",
            _ => "低",
        };
        let risk_desc = risk.get("risk").and_then(|v| v.as_str()).unwrap_or("");
        lines.push(format!("- [{}] {}", severity_cn, risk_desc));
        let suggestion = risk
            .get("suggestion")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        lines.push(format!("  - 建议: {}\n", suggestion));
    }

    lines.push("## Raw LLM Output (前2000字)\n\n```".to_string());
    let preview: String = raw_output.chars().take(2000).collect();
    lines.push(preview);
    lines.push("```".to_string());

    let report_path = project_dir
        .join("outputs")
        .join("reports")
        .join(format!("{}_memory_update_report.md", chapter_id));
    write_text(&report_path, &lines.join("\n"));
}

fn do_update_memory(app: &tauri::AppHandle, chapter_id: &str) -> CommandResult {
    let config = match build_run_config(app) {
        Ok(c) => c,
        Err(e) => return CommandResult { success: false, log: e },
    };
    let settings = load_settings(app);

    let chapter_path = config
        .project_dir
        .join("chapters")
        .join(chapter_id)
        .join("chapter.md");

    if !chapter_path.exists() {
        return CommandResult {
            success: false,
            log: format!(
                "[错误] 未找到 {}\n[提示] 请先生成该章节。",
                chapter_path.display()
            ),
        };
    }

    let chapter_text = read_text(&chapter_path);
    if chapter_text.trim().is_empty() {
        return CommandResult {
            success: false,
            log: format!("[错误] {} 是空文件", chapter_path.display()),
        };
    }

    let mut log = String::new();
    log.push_str(&format!(
        "[信息] 正在读取 {}/chapter.md...\n",
        chapter_id
    ));
    log.push_str(&format!(
        "[信息] 章节文本长度: {} 字符\n",
        chapter_text.chars().count()
    ));

    // 读取现有角色和伏笔
    let characters =
        load_yaml(&config.project_dir.join("memory").join("characters.yaml"));
    let foreshadowing =
        load_yaml(&config.project_dir.join("memory").join("foreshadowing.yaml"));

    let characters_text = serde_json::to_string_pretty(
        &serde_json::from_str::<serde_json::Value>(
            &serde_yaml::to_string(&characters).unwrap_or_default(),
        )
        .unwrap_or(serde_json::Value::Null),
    )
    .unwrap_or_default();
    let foreshadowing_text = serde_json::to_string_pretty(
        &serde_json::from_str::<serde_json::Value>(
            &serde_yaml::to_string(&foreshadowing).unwrap_or_default(),
        )
        .unwrap_or(serde_json::Value::Null),
    )
    .unwrap_or_default();

    let prompt = EXTRACTION_PROMPT_TEMPLATE
        .replace("{chapter_id}", chapter_id)
        .replace("{chapter_text}", &chapter_text)
        .replace("{characters_text}", &characters_text)
        .replace("{foreshadowing_text}", &foreshadowing_text);

    let messages = sys_user_messages(
        "你只输出严格合法的JSON，不输出任何其他文本。",
        &apply_no_think(prompt.clone(), &settings),
    );

    let top_p = settings.top_p.unwrap_or(0.85);

    log.push_str("[信息] 正在调用模型进行记忆抽取...\n");

    // 记忆抽取必须吐出完整 JSON，绝不能被输出上限截断：
    // 这里不发送 max_tokens，让模型写到自然结束（仅受上下文窗口约束）。
    let raw_output = match llm::call_llm(
        &config.llm_base_url,
        &config.llm_api_key,
        &config.model_name,
        messages,
        0.3,
        top_p,
        None,
    ) {
        Ok(r) => r,
        Err(e) => {
            return CommandResult {
                success: false,
                log: format!("{}[错误] 模型调用失败: {}", log, e),
            };
        }
    };

    let data = match parse_extraction_json(&raw_output) {
        Ok(d) => d,
        Err(e) => {
            let fail_path = config
                .project_dir
                .join("outputs")
                .join("reports")
                .join(format!("{}_memory_parse_failed.md", chapter_id));
            write_text(
                &fail_path,
                &format!("# 记忆抽取 JSON 解析失败\n\n{}", raw_output),
            );
            return CommandResult {
                success: false,
                log: format!(
                    "{}[错误] {}\n[信息] 原始输出已保存到: {}",
                    log,
                    e,
                    fail_path.display()
                ),
            };
        }
    };

    log.push_str("[成功] JSON 解析成功\n");
    log.push_str("[信息] 正在应用记忆更新...\n");

    apply_memory_updates(&data, chapter_id, &config.project_dir);
    generate_memory_report(&data, chapter_id, &raw_output, &config.project_dir);

    log.push_str(&format!(
        "\n[完成] {} 的记忆更新全部完成！",
        chapter_id
    ));

    CommandResult {
        success: true,
        log: log.trim().to_string(),
    }
}

// ============================================================
// 4.4 reset_chapter（移植 reset_chapter.py）
// ============================================================

fn belongs_to_chapter(record: &serde_json::Value, chapter_id: &str) -> bool {
    if let Some(cid) = record.get("chapter_id").and_then(|v| v.as_str()) {
        if cid == chapter_id {
            return true;
        }
    }
    if let Some(eid) = record.get("event_id").and_then(|v| v.as_str()) {
        if eid.starts_with(&format!("{}_", chapter_id)) {
            return true;
        }
    }
    false
}

fn filter_memory_records(chapter_id: &str, project_dir: &Path) -> String {
    let targets = [
        project_dir.join("memory").join("chapter_summaries.jsonl"),
        project_dir.join("memory").join("events.jsonl"),
        project_dir.join("memory").join("timeline.jsonl"),
    ];
    let mut log = String::new();
    for path in &targets {
        let records = read_jsonl(path);
        let original_count = records.len();
        let kept: Vec<serde_json::Value> = records
            .into_iter()
            .filter(|r| !belongs_to_chapter(r, chapter_id))
            .collect();
        let removed = original_count - kept.len();
        rewrite_jsonl(path, &kept);
        log.push_str(&format!(
            "[记忆] {}: 删除 {} 条记录\n",
            path.file_name().unwrap_or_default().to_string_lossy(),
            removed
        ));
    }
    log
}

fn do_reset_chapter(
    app: &tauri::AppHandle,
    chapter_id: &str,
    include_memory: bool,
) -> CommandResult {
    let config = match build_run_config(app) {
        Ok(c) => c,
        Err(e) => return CommandResult { success: false, log: e },
    };

    let mut log = String::new();

    // 删除章节目录
    let chapter_dir = config.project_dir.join("chapters").join(chapter_id);
    if chapter_dir.exists() {
        if let Err(e) = fs::remove_dir_all(&chapter_dir) {
            log.push_str(&format!("[错误] 删除 {} 失败: {}\n", chapter_dir.display(), e));
        } else {
            log.push_str(&format!("[删除] {}\n", chapter_dir.display()));
        }
    } else {
        log.push_str(&format!(
            "[跳过] 章节目录不存在: {}\n",
            chapter_dir.display()
        ));
    }

    // 删除报告文件
    let reports_dir = config.project_dir.join("outputs").join("reports");
    let mut deleted_reports = 0u32;
    if let Ok(entries) = fs::read_dir(&reports_dir) {
        let prefix = format!("{}_", chapter_id);
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with(&prefix) {
                        if fs::remove_file(&path).is_ok() {
                            deleted_reports += 1;
                            log.push_str(&format!("[删除] {}\n", path.display()));
                        }
                    }
                }
            }
        }
    }
    log.push_str(&format!(
        "[完成] 已删除 {} 个报告文件\n",
        deleted_reports
    ));

    // 处理记忆过滤
    if include_memory {
        log.push_str(&filter_memory_records(chapter_id, &config.project_dir));
    } else {
        log.push_str("[信息] 未删除长期记忆。需要时可勾选「同时清除记忆」。\n");
    }

    CommandResult {
        success: true,
        log: log.trim().to_string(),
    }
}

// ============================================================
// 简单 Tauri 命令（文件读写 / 设置 / 列表）
// ============================================================

#[tauri::command]
fn get_pipeline_root(app: tauri::AppHandle) -> Result<String, String> {
    let config = build_run_config(&app)?;
    Ok(config.project_dir.to_string_lossy().to_string())
}

#[tauri::command]
fn list_chapters(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let config = build_run_config(&app)?;
    let chapters_dir = config.project_dir.join("chapters");
    let mut ids: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(&chapters_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    ids.push(name.to_string());
                }
            }
        }
    }
    ids.sort();
    Ok(ids)
}

#[tauri::command]
fn read_chapter(app: tauri::AppHandle, chapter_id: String) -> Result<String, String> {
    let config = build_run_config(&app)?;
    let path = config
        .project_dir
        .join("chapters")
        .join(&chapter_id)
        .join("chapter.md");
    Ok(read_text(&path))
}

#[tauri::command]
fn save_chapter(
    app: tauri::AppHandle,
    chapter_id: String,
    content: String,
) -> Result<(), String> {
    let config = build_run_config(&app)?;
    let dir = config.project_dir.join("chapters").join(&chapter_id);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    fs::write(dir.join("chapter.md"), content).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_report(
    app: tauri::AppHandle,
    chapter_id: String,
    kind: String,
) -> Result<String, String> {
    let config = build_run_config(&app)?;
    let filename = match kind.as_str() {
        "generation" => format!("{}_chapter_generation_report.md", chapter_id),
        "consistency" => format!("{}_consistency_report.md", chapter_id),
        "memory" => format!("{}_memory_update_report.md", chapter_id),
        _ => return Err(format!("未知报告类型: {}", kind)),
    };
    let path = config
        .project_dir
        .join("outputs")
        .join("reports")
        .join(filename);
    Ok(read_text(&path))
}

#[tauri::command]
fn read_memory_file(app: tauri::AppHandle, rel: String) -> Result<String, String> {
    let rel = normalize_rel(&rel);
    if !EDITABLE_FILES.contains(&rel.as_str()) {
        return Err(format!("不允许读取该文件: {}", rel));
    }
    let config = build_run_config(&app)?;
    Ok(read_text(&config.project_dir.join(&rel)))
}

#[tauri::command]
fn save_memory_file(
    app: tauri::AppHandle,
    rel: String,
    content: String,
) -> Result<(), String> {
    let rel = normalize_rel(&rel);
    if !EDITABLE_FILES.contains(&rel.as_str()) {
        return Err(format!("不允许写入该文件: {}", rel));
    }
    let config = build_run_config(&app)?;
    let path = config.project_dir.join(&rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, content).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> Result<Settings, String> {
    let config = build_run_config(&app)?;
    let mut settings = load_settings(&app);
    // 确保 project_dir 回填（展示当前实际使用的目录）
    if settings.project_dir.trim().is_empty() {
        settings.project_dir = config.project_dir.to_string_lossy().to_string();
    }
    Ok(settings)
}

#[tauri::command]
fn save_settings(app: tauri::AppHandle, settings: Settings) -> Result<(), String> {
    // 确保项目目录与首次初始化就绪（不依赖其返回值）
    build_run_config(&app)?;
    let path = settings_path(&app);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let text = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}

#[tauri::command]
async fn generate_chapter(
    app: tauri::AppHandle,
    chapter_id: String,
    idea: String,
    target_words: u32,
    use_context: bool,
    overwrite: bool,
    pov: Option<String>,
    narrative: Option<String>,
) -> Result<CommandResult, String> {
    let pov = pov.unwrap_or_default();
    let narrative = narrative.unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || {
        do_generate_chapter(
            &app,
            &chapter_id,
            &idea,
            target_words,
            use_context,
            overwrite,
            &pov,
            &narrative,
        )
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn check_consistency(
    app: tauri::AppHandle,
    chapter_id: String,
) -> Result<CommandResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        do_check_consistency(&app, &chapter_id)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_memory(
    app: tauri::AppHandle,
    chapter_id: String,
) -> Result<CommandResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        do_update_memory(&app, &chapter_id)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn reset_chapter(
    app: tauri::AppHandle,
    chapter_id: String,
    include_memory: bool,
) -> Result<CommandResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        do_reset_chapter(&app, &chapter_id, include_memory)
    })
    .await
    .map_err(|e| e.to_string())
}

// ============================================================
// 应用入口
// ============================================================

#[tauri::command]
fn open_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // 运行时显式设置窗口/任务栏图标，确保 dev 与正式版都显示新 logo
            // include_image! 在编译期嵌入；图标文件变化会强制重新编译。
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.set_icon(tauri::include_image!("icons/icon.png"));
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_pipeline_root,
            list_chapters,
            read_chapter,
            save_chapter,
            read_report,
            read_memory_file,
            save_memory_file,
            get_settings,
            save_settings,
            generate_chapter,
            check_consistency,
            update_memory,
            reset_chapter,
            open_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ============================================================
// 测试：覆盖格式关键逻辑 + LLM HTTP 链路（进程内 mock，不依赖外部服务/Python）
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::PathBuf;

    fn temp_project(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut dir = std::env::temp_dir();
        dir.push(format!("novelpipe_test_{}_{}_{}", tag, std::process::id(), nanos));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("memory")).unwrap();
        dir
    }

    #[test]
    fn clean_output_strips_prefix_and_fences() {
        assert_eq!(clean_output("以下是正文：\n正文开始"), "正文开始");
        assert_eq!(clean_output("```text\n正文内容\n```"), "正文内容");
        assert_eq!(clean_output("纯正文"), "纯正文");
    }

    #[test]
    fn parse_extraction_json_handles_messy_output() {
        // 带 ```json 围栏
        let v = parse_extraction_json("```json\n{\"a\":1}\n```").unwrap();
        assert_eq!(v["a"], 1);
        // 前缀文字 + 尾部多余内容
        let v = parse_extraction_json("好的，结果如下：{\"b\":2} 以上。").unwrap();
        assert_eq!(v["b"], 2);
        // 非法 JSON 返回错误
        assert!(parse_extraction_json("这不是 JSON").is_err());
    }

    #[test]
    fn generation_report_idea_roundtrip() {
        // 写生成报告后，一致性检查必须能从中解析回 idea（跨文件依赖）
        let dir = temp_project("report");
        let cfg = RunConfig {
            project_dir: dir.clone(),
            llm_base_url: "x".into(),
            llm_api_key: "x".into(),
            model_name: "test-model".into(),
            max_output_tokens: Some(100),
        };
        let idea = "主角回到故乡，发现父亲留下的一封信，决定调查旧事。";
        write_generation_report(
            &dir, "ch001", idea, 4000, true, 123, "正文内容", "raw resp", "user prompt", &[], &cfg,
        );
        assert_eq!(load_chapter_idea(&dir, "ch001"), idea);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_memory_updates_roundtrip_formats() {
        let dir = temp_project("mem");
        write_text(
            &dir.join("memory").join("characters.yaml"),
            "characters:\n  - name: 林川\n    current_status: 在故乡\n    knows: []\n",
        );

        let data = json!({
            "chapter_id": "ch001",
            "chapter_summary": "主角回到故乡。",
            "events": [{
                "event_id": "ch001_evt001",
                "type": "discovery",
                "characters": ["林川"],
                "description": "发现父亲的信",
                "location": "老宅",
                "time_order": "傍晚",
                "importance": 8
            }],
            "character_updates": [
                {"name": "林川", "field": "current_status", "new_value": "决定调查旧事"},
                {"name": "林川", "field": "knows", "new_value": "父亲留下了一封信"}
            ],
            "foreshadowing_updates": [
                {"id": "fs001", "action": "add", "description": "信里提到的名字",
                 "planned_resolution": "后续揭示", "related_characters": ["林川"]}
            ],
            "relationship_updates": [
                {"source": "林川", "target": "父亲", "relation": "父子", "change": "通过信件重新连接"}
            ]
        });
        apply_memory_updates(&data, "ch001", &dir);

        // 章节摘要
        let summaries = read_jsonl(&dir.join("memory").join("chapter_summaries.jsonl"));
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0]["chapter_id"], "ch001");
        assert_eq!(summaries[0]["chapter_summary"], "主角回到故乡。");

        // 事件：补上了 chapter_id
        let events = read_jsonl(&dir.join("memory").join("events.jsonl"));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["chapter_id"], "ch001");
        assert_eq!(events[0]["description"], "发现父亲的信");

        // 时间线：time 取自 time_order
        let timeline = read_jsonl(&dir.join("memory").join("timeline.jsonl"));
        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline[0]["event_id"], "ch001_evt001");
        assert_eq!(timeline[0]["time"], "傍晚");

        // 角色：current_status 覆盖；knows 追加
        let chars = load_yaml(&dir.join("memory").join("characters.yaml"));
        let c = &chars.get("characters").unwrap().as_sequence().unwrap()[0];
        assert_eq!(c.get("current_status").unwrap().as_str(), Some("决定调查旧事"));
        let knows = c.get("knows").unwrap().as_sequence().unwrap();
        assert!(knows.iter().any(|v| v.as_str() == Some("父亲留下了一封信")));

        // 伏笔：新增到 active
        let fsd = load_yaml(&dir.join("memory").join("foreshadowing.yaml"));
        let active = fsd.get("active").unwrap().as_sequence().unwrap();
        assert!(active
            .iter()
            .any(|f| f.get("id").and_then(|v| v.as_str()) == Some("fs001")));

        // 关系图：新增边 林川 -> 父亲
        let rel = load_json(&dir.join("memory").join("relationships.json"));
        let edges = rel.get("edges").unwrap().as_array().unwrap();
        assert!(edges
            .iter()
            .any(|e| e["source"] == "林川" && e["target"] == "父亲"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn repair_recovers_truncated_extraction_json() {
        // 模拟模型在 events 第二个元素中途被 token 上限截断
        let truncated = "{\n  \"chapter_summary\": \"测试摘要\",\n  \"events\": [\n    {\"event_id\": \"ch001_evt001\", \"description\": \"完整事件\"},\n    {\"event_id\": \"ch001_evt002\",\n     \"chapte";
        // 严格解析必然失败
        assert!(serde_json::from_str::<serde_json::Value>(truncated).is_err());
        // 抢救后应能解析，且保住摘要与已完整的首个事件
        let v = parse_extraction_json(truncated).expect("应能抢救截断的 JSON");
        assert_eq!(v["chapter_summary"], "测试摘要");
        let events = v["events"].as_array().unwrap();
        assert!(!events.is_empty());
        assert_eq!(events[0]["event_id"], "ch001_evt001");
        assert_eq!(events[0]["description"], "完整事件");
    }

    #[test]
    fn belongs_to_chapter_matches_id_and_event_prefix() {
        assert!(belongs_to_chapter(&json!({"chapter_id": "ch002"}), "ch002"));
        assert!(belongs_to_chapter(&json!({"event_id": "ch002_evt003"}), "ch002"));
        assert!(!belongs_to_chapter(&json!({"chapter_id": "ch001"}), "ch002"));
    }

    #[test]
    fn llm_call_against_mock_server() {
        // 进程内 mock OpenAI 兼容服务，验证 HTTP 请求与响应解析全链路
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let body = r#"{"choices":[{"message":{"content":"测试输出"}}]}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.as_bytes().len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes());
                let _ = stream.flush();
            }
        });

        let base = format!("http://127.0.0.1:{}/v1", port);
        let msgs = vec![llm::ChatMessage {
            role: "user".into(),
            content: "hi".into(),
        }];
        let out = llm::call_llm(&base, "k", "m", msgs, 0.7, 0.9, Some(50)).unwrap();
        assert_eq!(out, "测试输出");
        let _ = handle.join();
    }

    #[test]
    fn llm_call_normalizes_missing_scheme() {
        // 用户漏写 http:// 时应自动补全，而不是报 builder error
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let body = r#"{"choices":[{"message":{"content":"ok"}}]}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.as_bytes().len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes());
            }
        });
        // 注意：没有 http:// 前缀
        let base = format!("127.0.0.1:{}", port);
        let msgs = vec![llm::ChatMessage {
            role: "user".into(),
            content: "hi".into(),
        }];
        let out = llm::call_llm(&base, "k", "m", msgs, 0.7, 0.9, Some(50)).unwrap();
        assert_eq!(out, "ok");
        let _ = handle.join();
    }
}
