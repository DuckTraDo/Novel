"""
build_context.py
根据 chapter_id 和 scene_id 生成当前场景的写作上下文。

用法:
    python scripts/build_context.py --chapter ch001 --scene scene001
"""

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from utils import (
    get_project_root, ensure_dirs, load_config,
    load_yaml, load_json, read_jsonl, read_text,
    write_text, simple_token_estimate, get_scene_ending,
)


def build_context(chapter_id: str, scene_id: str) -> str:
    """构建当前场景的写作上下文，返回 markdown 字符串"""
    root = get_project_root()
    config = load_config()

    # 加载所有数据源
    story_bible = load_yaml(root / "memory" / "story_bible.yaml")
    characters_data = load_yaml(root / "memory" / "characters.yaml")
    foreshadowing = load_yaml(root / "memory" / "foreshadowing.yaml")
    chapter_outlines = load_yaml(root / "outlines" / "chapter_outlines.yaml")
    events = read_jsonl(root / "memory" / "events.jsonl")
    chapter_summaries = read_jsonl(root / "memory" / "chapter_summaries.jsonl")
    style_bank = read_jsonl(root / "memory" / "style_bank.jsonl")
    prev_ending = get_scene_ending(chapter_id, scene_id)

    sections = []
    sections.append("# Writing Context\n")

    # 1. Story Bible
    sections.append(format_story_bible(story_bible))

    # 2. Characters
    sections.append(format_characters(characters_data))

    # 3. Chapter Outline & Scene Goal
    ch_outlines = chapter_outlines.get("chapters", {})
    chapter = ch_outlines.get(chapter_id, {})
    sections.append(format_chapter_outline(chapter, chapter_id))
    sections.append(format_scene_goal(chapter, scene_id))

    # 4. Previous Scene Ending
    sections.append(format_prev_ending(prev_ending))

    # 5. Recent Chapter Summaries (最近 5 条)
    sections.append(format_summaries(chapter_summaries[-5:]))

    # 6. Active Foreshadowing
    sections.append(format_foreshadowing(foreshadowing))

    # 7. Recent Events (最近 20 条)
    sections.append(format_events(events[-20:]))

    # 8. Style References (前 3 条)
    sections.append(format_style_refs(style_bank[:3]))

    # 9. Writing Instructions
    sections.append(format_instructions(config))

    return "\n".join(sections)


def format_story_bible(bible: dict) -> str:
    lines = ["## Story Bible\n"]
    lines.append(f"**标题**: {bible.get('title', '未命名')}")
    lines.append(f"**类型**: {bible.get('genre', '未知')}")
    lines.append(f"**基调**: {bible.get('tone', '未定')}")

    for rule in bible.get("world_rules", []):
        lines.append(f"- {rule}")

    writing_rules = bible.get("writing_rules", [])
    if writing_rules:
        lines.append("\n### 写作规则")
        for rule in writing_rules:
            lines.append(f"- {rule}")

    forbidden = bible.get("forbidden_patterns", [])
    if forbidden:
        lines.append("\n### 禁止模式")
        for p in forbidden:
            lines.append(f"- {p}")

    return "\n".join(lines)


def format_characters(chars_data: dict) -> str:
    characters = chars_data.get("characters", [])
    if not characters:
        return "## Relevant Characters\n\n（无角色数据）"

    lines = ["## Relevant Characters\n"]
    for char in characters:
        lines.append(f"### {char.get('name', '未知')} ({char.get('role', '未知')})")
        lines.append(f"- 身份: {char.get('identity', '')}")
        lines.append(f"- 性格: {char.get('personality', '')}")
        lines.append(f"- 说话风格: {char.get('speaking_style', '')}")
        lines.append(f"- 当前状态: {char.get('current_status', '')}")
        secrets = char.get("secrets", [])
        if secrets:
            lines.append(f"- 秘密: {'; '.join(secrets)}")
        knows = char.get("knows", [])
        if knows:
            lines.append(f"- 已知信息: {'; '.join(knows)}")
        constraints = char.get("constraints", [])
        if constraints:
            lines.append(f"- 限制: {'; '.join(constraints)}")
        lines.append("")
    return "\n".join(lines)


def format_chapter_outline(chapter: dict, chapter_id: str) -> str:
    if not chapter:
        return f"## Current Chapter Outline\n\n（未找到 {chapter_id} 的大纲）"
    lines = ["## Current Chapter Outline\n"]
    lines.append(f"**章节标题**: {chapter.get('title', '未命名')}")
    lines.append(f"**POV**: {chapter.get('pov', '未知')}")
    lines.append(f"**概要**: {chapter.get('summary', '')}")
    return "\n".join(lines)


def format_scene_goal(chapter: dict, scene_id: str) -> str:
    if not chapter:
        return ""
    scenes = chapter.get("scenes", {})
    scene = scenes.get(scene_id, {})
    if not scene:
        return f"## Current Scene Goal\n\n（未找到 {scene_id} 的大纲）"

    lines = ["## Current Scene Goal\n"]
    lines.append(f"**场景标题**: {scene.get('title', '')}")
    lines.append(f"**目标**: {scene.get('goal', '')}")
    lines.append(f"**地点**: {scene.get('location', '')}")
    lines.append(f"**角色**: {', '.join(scene.get('characters', []))}")
    beats = scene.get("key_beats", [])
    if beats:
        lines.append("\n**关键节拍**:")
        for beat in beats:
            lines.append(f"- {beat}")
    conflict = scene.get("conflict", "")
    if conflict:
        lines.append(f"\n**冲突**: {conflict}")
    return "\n".join(lines)


def format_prev_ending(prev_ending: str) -> str:
    if not prev_ending:
        return "## Previous Scene Ending\n\n（这是本章第一个场景，无前文）"
    return f"## Previous Scene Ending\n\n```\n{prev_ending}\n```"


def format_summaries(summaries: list) -> str:
    if not summaries:
        return "## Recent Chapter Summaries\n\n（暂无章节摘要）"
    lines = ["## Recent Chapter Summaries\n"]
    for s in summaries:
        lines.append(f"- **{s.get('chapter_id', '?')}**: {s.get('chapter_summary', '')}")
    return "\n".join(lines)


def format_foreshadowing(foreshadowing: dict) -> str:
    active = foreshadowing.get("active", [])
    if not active:
        return "## Active Foreshadowing\n\n（无活跃伏笔）"
    lines = ["## Active Foreshadowing\n"]
    for fs in active:
        lines.append(f"- **{fs.get('id', '?')}** [{fs.get('introduced_in', '?')}]: {fs.get('description', '')}")
        lines.append(f"  相关角色: {', '.join(fs.get('related_characters', []))}")
        lines.append(f"  计划回收: {fs.get('planned_resolution', '')}")
    return "\n".join(lines)


def format_events(events: list) -> str:
    if not events:
        return "## Relevant Past Events\n\n（暂无事件记录）"
    lines = ["## Relevant Past Events\n"]
    for evt in events:
        lines.append(f"- **{evt.get('event_id', '?')}** ({evt.get('type', '?')}): {evt.get('description', '')}")
        lines.append(f"  人物: {', '.join(evt.get('characters', []))}")
    return "\n".join(lines)


def format_style_refs(style_refs: list) -> str:
    if not style_refs:
        return "## Style References\n\n（暂无风格参考）"
    lines = ["## Style References\n"]
    for ref in style_refs:
        lines.append(f"**{ref.get('id', '?')}** ({ref.get('source', '')}):")
        lines.append(f"> {ref.get('text', '')}\n")
    return "\n".join(lines)


def format_instructions(config: dict) -> str:
    lines = ["## Writing Instructions\n"]
    lines.append("请根据以上上下文，写出当前场景的正文。要求：")
    lines.append("- 只写正文，不要写标题、分析、总结或解释")
    lines.append("- 不要以'以下是'、'下面是'等开头")
    lines.append("- 保持人物语气一致")
    lines.append("- 遵守世界观规则和写作规则")
    lines.append("- 如果上下文没有明确事实，不要编造会破坏设定的大事件")
    lines.append("- 输出中文小说正文")
    style_rules = config.get("default_style_rules", [])
    if style_rules:
        lines.append("\n### 风格要求")
        for rule in style_rules:
            lines.append(f"- {rule}")
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description="生成场景写作上下文")
    parser.add_argument("--chapter", required=True, help="章节 ID，如 ch001")
    parser.add_argument("--scene", required=True, help="场景 ID，如 scene001")
    args = parser.parse_args()

    ensure_dirs()
    print(f"[信息] 正在为 {args.chapter}/{args.scene} 构建上下文...")

    context = build_context(args.chapter, args.scene)

    root = get_project_root()
    output_path = root / "outputs" / "contexts" / f"{args.chapter}_{args.scene}_context.md"
    write_text(output_path, context)

    est = simple_token_estimate(context)
    config = load_config()
    max_tokens = config.get("max_context_tokens", 30000)
    print(f"[完成] 上下文已保存到: {output_path}")
    print(f"[信息] 上下文估算 token 数: ~{est} / {max_tokens}")
    if est > max_tokens:
        print(f"[警告] 上下文超出限制！考虑缩减内容。")


if __name__ == "__main__":
    main()
