"""
generate_chapter_local.py
根据一句 chapter idea 和长期记忆，一次性生成完整章节正文。

用法:
    python scripts/generate_chapter_local.py --chapter ch001 --idea "这一章大概发生什么。"
    python scripts/generate_chapter_local.py --chapter ch001 --idea-file inputs/ch001_idea.txt
"""

import argparse
import json
import re
import sys
from datetime import datetime
from pathlib import Path
from typing import Any

import yaml

sys.path.insert(0, str(Path(__file__).resolve().parent))

from utils import (
    get_llm_config,
    get_project_root,
    ensure_dirs,
    load_config,
    load_yaml,
    read_jsonl,
    read_text,
    write_text,
    call_local_llm,
    simple_token_estimate,
    configure_utf8_stdio,
)


SYSTEM_PROMPT = """你是中文长篇小说写手。

必须遵守：
- 只输出小说正文
- 不要解释
- 不要输出大纲
- 不要输出分析
- 不要写“以下是”
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
- 输出中文小说正文"""


def dump_block(value: Any) -> str:
    """把结构化数据转成适合 prompt 的文本。"""
    if value in ({}, [], None, ""):
        return "（未提供）"
    return yaml.safe_dump(value, allow_unicode=True, sort_keys=False).strip()


def format_jsonl_records(records: list[dict], empty_text: str) -> str:
    if not records:
        return empty_text
    lines = []
    for item in records:
        lines.append(json.dumps(item, ensure_ascii=False))
    return "\n".join(lines)


def format_style_references(records: list[dict]) -> str:
    if not records:
        return "（未提供）"
    parts = []
    for item in records:
        sid = item.get("id", "style")
        text = item.get("text", "")
        if text:
            parts.append(f"[{sid}]\n{text}")
    return "\n\n".join(parts) if parts else "（未提供）"


def extract_idea(args: argparse.Namespace) -> str:
    if args.idea:
        idea = args.idea.strip()
    else:
        idea_path = Path(args.idea_file)
        if not idea_path.is_absolute():
            idea_path = get_project_root() / idea_path
        if not idea_path.exists():
            print(f"[错误] idea 文件不存在: {idea_path}")
            sys.exit(1)
        idea = read_text(idea_path).strip()

    if not idea:
        print("[错误] chapter idea 不能为空")
        sys.exit(1)
    return idea


def build_memory_sections(use_context: bool) -> tuple[dict[str, str], list[tuple[str, str]]]:
    root = get_project_root()
    if not use_context:
        skipped = "（--no-context 已启用，未读取长期记忆）"
        return {
            "Story Bible": skipped,
            "Characters": skipped,
            "Book Outline": skipped,
            "Recent Chapter Summaries": skipped,
            "Recent Events": skipped,
            "Timeline": skipped,
            "Active Foreshadowing": skipped,
            "Style References": skipped,
        }, []

    story_bible = load_yaml(root / "memory" / "story_bible.yaml")
    characters = load_yaml(root / "memory" / "characters.yaml")
    foreshadowing = load_yaml(root / "memory" / "foreshadowing.yaml")
    book_outline = load_yaml(root / "outlines" / "book_outline.yaml")
    chapter_summaries = read_jsonl(root / "memory" / "chapter_summaries.jsonl")[-5:]
    events = read_jsonl(root / "memory" / "events.jsonl")[-20:]
    timeline = read_jsonl(root / "memory" / "timeline.jsonl")[-20:]
    style_bank = read_jsonl(root / "memory" / "style_bank.jsonl")[:5]

    sections = {
        "Story Bible": dump_block(story_bible),
        "Characters": dump_block(characters),
        "Book Outline": dump_block(book_outline),
        "Recent Chapter Summaries": format_jsonl_records(chapter_summaries, "（暂无章节摘要）"),
        "Recent Events": format_jsonl_records(events, "（暂无事件记录）"),
        "Timeline": format_jsonl_records(timeline, "（暂无时间线记录）"),
        "Active Foreshadowing": dump_block(foreshadowing),
        "Style References": format_style_references(style_bank),
    }
    conflict_sources = [
        ("Story Bible", sections["Story Bible"]),
        ("Characters", sections["Characters"]),
        ("Book Outline", sections["Book Outline"]),
        ("Recent Chapter Summaries", sections["Recent Chapter Summaries"]),
        ("Recent Events", sections["Recent Events"]),
        ("Timeline", sections["Timeline"]),
        ("Active Foreshadowing", sections["Active Foreshadowing"]),
    ]
    return sections, conflict_sources


def idea_terms(idea: str) -> set[str]:
    terms: set[str] = set()
    for chunk in re.findall(r"[\u4e00-\u9fffA-Za-z0-9_]{2,}", idea):
        if len(chunk) <= 6:
            terms.add(chunk)
        for i in range(max(0, len(chunk) - 1)):
            terms.add(chunk[i:i + 2])
    return terms


def detect_potential_conflicts(idea: str, sources: list[tuple[str, str]]) -> list[str]:
    """轻量字面提示，不替代作者审稿。"""
    terms = idea_terms(idea)
    if not terms:
        return []

    markers = (
        "年龄", "岁", "天气", "地点", "位置", "金额", "价格", "物品", "身份",
        "状态", "死亡", "失踪", "禁止", "不能", "不要", "秘密", "已", "已经",
        "任务", "奖励", "发现",
    )
    warnings: list[str] = []
    for label, text in sources:
        for raw_line in text.splitlines():
            line = raw_line.strip()
            if len(line) < 6 or not any(marker in line for marker in markers):
                continue
            overlap = [term for term in terms if len(term) >= 2 and term in line]
            if len(overlap) >= 2:
                warnings.append(f"- [{label}] {line[:180]}")
                if len(warnings) >= 8:
                    return warnings
    return warnings


def build_user_prompt(chapter_id: str, idea: str, target_words: int, sections: dict[str, str]) -> str:
    return f"""# Chapter Generation Context

## Chapter ID
{chapter_id}

## Author Chapter Idea
{idea}

## Target Length
约 {target_words} 中文字

## Story Bible
{sections["Story Bible"]}

## Characters
{sections["Characters"]}

## Book Outline
{sections["Book Outline"]}

## Recent Chapter Summaries
{sections["Recent Chapter Summaries"]}

## Recent Events
{sections["Recent Events"]}

## Timeline
{sections["Timeline"]}

## Active Foreshadowing
{sections["Active Foreshadowing"]}

## Style References
{sections["Style References"]}

## Hard Rules For This Chapter
- 只写本章正文
- 不要写章节分析
- 不要输出 markdown 标题
- 不要重复前文已发生事件
- 不要在本章内部重复同一个发现、奖励、任务、冲突或转折
- 结尾可以留钩子，但不要提前进入下一章主要剧情"""


def clean_output(raw: str) -> str:
    text = raw.strip()
    prefixes = [
        "以下是小说正文：",
        "以下是正文：",
        "下面是小说正文：",
        "下面是正文：",
        "好的，",
        "好的。",
    ]
    for prefix in prefixes:
        if text.startswith(prefix):
            text = text[len(prefix):].strip()
    if text.startswith("```"):
        first_newline = text.find("\n")
        if first_newline != -1:
            text = text[first_newline + 1:]
        if text.rstrip().endswith("```"):
            text = text.rstrip()[:-3].rstrip()
    return text


def write_generation_report(
    chapter_id: str,
    idea: str,
    target_words: int,
    used_context: bool,
    context_tokens: int,
    output_text: str,
    raw_response: str,
    user_prompt: str,
    conflict_warnings: list[str],
    dry_run: bool,
):
    root = get_project_root()
    config = load_config()
    llm_cfg = get_llm_config()
    warning_text = "\n".join(conflict_warnings) if conflict_warnings else "未发现明显字面冲突；仍建议作者人工审阅长期记忆与本章 idea。"
    report_lines = [
        f"# Chapter Generation Report: {chapter_id}",
        "",
        "## Chapter ID",
        "",
        chapter_id,
        "",
        "## Idea",
        "",
        idea,
        "",
        "## Target Words",
        "",
        str(target_words),
        "",
        "## Used Context",
        "",
        str(used_context),
        "",
        "## Context Token Estimate",
        "",
        str(context_tokens),
        "",
        "## Potential Context Conflicts",
        "",
        warning_text,
        "",
        "## Output Length",
        "",
        f"{len(output_text)} characters",
        "",
        "## Model Info",
        "",
        f"- model: {llm_cfg.get('model', config.get('model_name', 'unknown'))}",
        f"- base_url: {llm_cfg.get('base_url', 'unknown')}",
        f"- temperature: {config.get('generation_temperature', 'N/A')}",
        f"- top_p: {config.get('generation_top_p', 'N/A')}",
        f"- dry_run: {dry_run}",
        "",
        "## Generation Time",
        "",
        datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
        "",
        "## Raw Response",
        "",
        "```text",
        raw_response,
        "```",
    ]
    if dry_run:
        report_lines.extend([
            "",
            "## Dry Run Prompt",
            "",
            "```text",
            user_prompt,
            "```",
        ])
    report_path = root / "outputs" / "reports" / f"{chapter_id}_chapter_generation_report.md"
    write_text(report_path, "\n".join(report_lines))
    print(f"[完成] 生成报告已保存: {report_path}")


def main():
    configure_utf8_stdio()
    parser = argparse.ArgumentParser(description="一次性生成完整章节正文")
    parser.add_argument("--chapter", required=True, help="章节 ID，如 ch001")
    idea_group = parser.add_mutually_exclusive_group(required=True)
    idea_group.add_argument("--idea", help="作者输入的一句或一段 chapter idea")
    idea_group.add_argument("--idea-file", help="从文件读取 chapter idea")
    parser.add_argument("--target-words", type=int, default=4000, help="目标中文字数，默认 4000")
    parser.add_argument("--overwrite", action="store_true", help="允许覆盖已存在的 chapters/<chapter>/chapter.md")
    parser.add_argument("--no-context", action="store_true", help="不读取长期记忆，只根据 idea 生成")
    parser.add_argument("--dry-run", action="store_true", help="只构建 prompt 和报告，不调用 LLM")
    args = parser.parse_args()

    ensure_dirs()
    chapter_id = args.chapter.strip()
    idea = extract_idea(args)
    root = get_project_root()
    chapter_path = root / "chapters" / chapter_id / "chapter.md"

    if chapter_path.exists() and not args.overwrite and not args.dry_run:
        print(f"[错误] {chapter_path} 已存在。使用 --overwrite 覆盖。")
        sys.exit(1)

    used_context = not args.no_context
    sections, conflict_sources = build_memory_sections(used_context)
    user_prompt = build_user_prompt(chapter_id, idea, args.target_words, sections)
    context_tokens = simple_token_estimate(user_prompt)
    conflict_warnings = detect_potential_conflicts(idea, conflict_sources) if used_context else []

    print(f"[信息] 章节: {chapter_id}")
    print(f"[信息] 目标长度: 约 {args.target_words} 中文字")
    print(f"[信息] 使用长期记忆: {used_context}")
    print(f"[信息] prompt token 估算: ~{context_tokens}")

    if args.dry_run:
        raw_response = "[DRY RUN] 未调用模型。"
        output_text = ""
        write_generation_report(
            chapter_id, idea, args.target_words, used_context, context_tokens,
            output_text, raw_response, user_prompt, conflict_warnings, dry_run=True,
        )
        print("[完成] dry-run 已完成，未调用 LLM，未写入 chapter.md")
        return

    messages = [
        {"role": "system", "content": SYSTEM_PROMPT},
        {"role": "user", "content": user_prompt},
    ]
    print("[信息] 正在调用本地模型生成完整章节...")
    raw_response = call_local_llm(messages)
    output_text = clean_output(raw_response)

    write_text(chapter_path, output_text)
    print(f"[完成] 章节正文已保存: {chapter_path}")

    write_generation_report(
        chapter_id, idea, args.target_words, used_context, context_tokens,
        output_text, raw_response, user_prompt, conflict_warnings, dry_run=False,
    )


if __name__ == "__main__":
    main()
