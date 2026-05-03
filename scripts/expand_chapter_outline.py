"""
expand_chapter_outline.py
将粗略的自然语言章节想法扩写成标准 chapter_outlines.yaml 结构。

用法:
    python scripts/expand_chapter_outline.py --chapter ch001 --idea "第一章：王二在肉铺下班..."
    python scripts/expand_chapter_outline.py --chapter ch001 --idea-file inputs/ch001_idea.txt
    python scripts/expand_chapter_outline.py --chapter ch001 --idea "..." --overwrite
    python scripts/expand_chapter_outline.py --chapter ch001 --idea "..." --append-scenes
    python scripts/expand_chapter_outline.py --chapter ch001 --idea "..." --num-scenes 4
"""

import argparse
import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from utils import (
    get_project_root, ensure_dirs, load_yaml, save_yaml,
    read_text, write_text, call_local_llm,
)


# ============================================================
# Prompt 模板
# ============================================================

EXPANSION_PROMPT_TEMPLATE = """你是一位专业的小说大纲编辑。请根据用户的粗略章节想法，扩写成结构化的章节大纲。

## 全书信息

### 书名
{book_title}

### 类型
{genre}

### 主题
{core_theme}

### 全书大纲
{book_outline}

## 角色档案

{characters_text}

## 活跃伏笔

{foreshadowing_text}

## 已有章节大纲（供参考上下文）

{existing_chapters_text}

## 用户的章节想法

{idea}

## 输出要求

请将上述想法扩写成 {num_scenes} 个场景，输出严格的 JSON 对象。不要包含任何其他文字、解释或 markdown 标记。

JSON schema：

```json
{{
  "chapter_id": "{chapter_id}",
  "title": "章节标题（2-6个字，有意象感）",
  "pov": "POV角色名",
  "summary": "用2-3句话概括本章内容",
  "scenes": {{
    "scene001": {{
      "title": "场景标题（2-4个字）",
      "goal": "这个场景要完成什么叙事目标",
      "location": "具体地点",
      "characters": ["出场角色1", "出场角色2"],
      "conflict": "场景核心冲突或张力",
      "ending_hook": "场景结尾的钩子/悬念/意象"
    }},
    "scene002": {{
      ...
    }}
  }}
}}
```

要求：
- scene 标题要简洁有意象，不要用"场景一"这种编号
- goal 要具体，不要写"推进情节"这种空话
- conflict 要来自人物处境，不要凭空制造戏剧性
- ending_hook 要有画面感或悬念，让人想读下去
- characters 只写实际出场的角色
- 保持与已有章节大纲的连贯性
- 符合全书的基调和主题

请直接输出 JSON。"""


# ============================================================
# 核心函数
# ============================================================

def build_expansion_context(chapter_id: str) -> dict:
    """读取所有参考材料"""
    root = get_project_root()

    book_outline = load_yaml(root / "outlines" / "book_outline.yaml")
    chapter_outlines = load_yaml(root / "outlines" / "chapter_outlines.yaml")
    story_bible = load_yaml(root / "memory" / "story_bible.yaml")
    characters = load_yaml(root / "memory" / "characters.yaml")
    foreshadowing = load_yaml(root / "memory" / "foreshadowing.yaml")

    # 格式化已有章节大纲（排除当前章节）
    existing_chapters = chapter_outlines.get("chapters", {})
    existing_lines = []
    for cid, cdata in existing_chapters.items():
        if cid != chapter_id:
            existing_lines.append(f"### {cid}: {cdata.get('title', '')}")
            existing_lines.append(f"POV: {cdata.get('pov', '')}")
            existing_lines.append(f"概要: {cdata.get('summary', '')}")
            scenes = cdata.get("scenes", {})
            for sid, sdata in scenes.items():
                existing_lines.append(f"  - {sid}: {sdata.get('title', '')} — {sdata.get('goal', '')}")
            existing_lines.append("")
    existing_chapters_text = "\n".join(existing_lines) if existing_lines else "（暂无已有章节）"

    # 格式化角色
    chars = characters.get("characters", [])
    chars_lines = []
    for c in chars:
        chars_lines.append(f"- **{c.get('name', '')}**（{c.get('role', '')}）: {c.get('identity', '')}")
        chars_lines.append(f"  性格: {c.get('personality', '')}")
        chars_lines.append(f"  说话: {c.get('speaking_style', '')}")
        chars_lines.append(f"  状态: {c.get('current_status', '')}")
        chars_lines.append("")
    characters_text = "\n".join(chars_lines) if chars_lines else "（暂无角色档案）"

    # 格式化伏笔
    active_fs = foreshadowing.get("active", [])
    fs_lines = []
    for fs in active_fs:
        fs_lines.append(f"- [{fs.get('id', '')}] {fs.get('description', '')}（相关: {', '.join(fs.get('related_characters', []))}）")
    foreshadowing_text = "\n".join(fs_lines) if fs_lines else "（暂无活跃伏笔）"

    return {
        "book_title": book_outline.get("title", "未命名"),
        "genre": book_outline.get("genre", ""),
        "core_theme": book_outline.get("core_theme", ""),
        "book_outline": format_book_outline(book_outline),
        "characters_text": characters_text,
        "foreshadowing_text": foreshadowing_text,
        "existing_chapters_text": existing_chapters_text,
    }


def format_book_outline(book_outline: dict) -> str:
    """格式化全书大纲为可读文本"""
    lines = []
    for key, val in book_outline.items():
        if key in ("title",):
            continue
        if isinstance(val, str):
            lines.append(f"{key}: {val}")
        elif isinstance(val, dict):
            lines.append(f"\n### {key}")
            for k2, v2 in val.items():
                if isinstance(v2, str):
                    lines.append(f"  {k2}: {v2}")
                elif isinstance(v2, dict):
                    lines.append(f"  {k2}:")
                    for k3, v3 in v2.items():
                        lines.append(f"    {k3}: {v3}")
        elif isinstance(val, list):
            lines.append(f"{key}:")
            for item in val:
                lines.append(f"  - {item}")
    return "\n".join(lines)


def expand_chapter(chapter_id: str, idea: str, num_scenes: int = 3) -> dict:
    """
    调用 LLM 将粗略想法扩写成结构化大纲。
    返回解析后的 JSON 字典。
    """
    ctx = build_expansion_context(chapter_id)

    prompt = EXPANSION_PROMPT_TEMPLATE.format(
        chapter_id=chapter_id,
        idea=idea,
        num_scenes=num_scenes,
        book_title=ctx["book_title"],
        genre=ctx["genre"],
        core_theme=ctx["core_theme"],
        book_outline=ctx["book_outline"],
        characters_text=ctx["characters_text"],
        foreshadowing_text=ctx["foreshadowing_text"],
        existing_chapters_text=ctx["existing_chapters_text"],
    )

    messages = [
        {"role": "system", "content": "你只输出严格合法的 JSON，不输出任何其他文本。"},
        {"role": "user", "content": prompt},
    ]

    print(f"[信息] 正在调用模型扩写 {chapter_id} 的大纲...")
    raw_output = call_local_llm(messages, temperature=0.5, top_p=0.9)

    return parse_json_output(raw_output, chapter_id), raw_output


def parse_json_output(raw_output: str, chapter_id: str) -> dict:
    """从模型输出中提取 JSON，带容错"""
    text = raw_output.strip()

    # 去掉 markdown 代码块
    json_match = re.search(r'```(?:json)?\s*\n?(.*?)\n?```', text, re.DOTALL)
    if json_match:
        text = json_match.group(1).strip()

    # 找到第一个 { 和最后一个 }
    if text and text[0] not in ('{', '['):
        start = text.find('{')
        if start != -1:
            text = text[start:]
    last_brace = text.rfind('}')
    if last_brace != -1:
        text = text[:last_brace + 1]

    try:
        data = json.loads(text)
    except json.JSONDecodeError as e:
        # 解析失败，保存原始输出
        root = get_project_root()
        fail_path = root / "outputs" / "reports" / f"{chapter_id}_outline_parse_failed.md"
        fail_path.parent.mkdir(parents=True, exist_ok=True)
        write_text(fail_path, f"# 大纲扩写 JSON 解析失败\n\n**章节**: {chapter_id}\n\n## 原始输出\n\n```\n{raw_output}\n```")
        print(f"[错误] JSON 解析失败: {e}")
        print(f"[信息] 原始输出已保存到: {fail_path}")
        sys.exit(1)

    return data


def write_to_chapter_outlines(data: dict, chapter_id: str, mode: str):
    """
    将扩写结果写入 chapter_outlines.yaml。

    mode:
        "overwrite" — 覆盖该章节
        "append" — 追加 scenes 到已有章节
        "new" — 新增章节（章节不存在时）
    """
    root = get_project_root()
    outlines_path = root / "outlines" / "chapter_outlines.yaml"
    outlines = load_yaml(outlines_path)
    if not outlines:
        outlines = {"chapters": {}}

    chapters = outlines.get("chapters", {})
    if chapters is None:
        chapters = {}

    if mode == "overwrite" or mode == "new":
        # 直接写入整个章节
        chapter_data = {
            "title": data.get("title", ""),
            "pov": data.get("pov", ""),
            "summary": data.get("summary", ""),
            "scenes": data.get("scenes", {}),
        }
        chapters[chapter_id] = chapter_data

    elif mode == "append":
        # 追加 scenes 到已有章节
        existing = chapters.get(chapter_id, {})
        existing_scenes = existing.get("scenes", {})
        if existing_scenes is None:
            existing_scenes = {}
        new_scenes = data.get("scenes", {})

        # 找到已有场景的最大编号，新场景从那里继续
        existing_nums = []
        for sid in existing_scenes:
            m = re.search(r'scene(\d+)', sid)
            if m:
                existing_nums.append(int(m.group(1)))
        next_num = max(existing_nums) + 1 if existing_nums else 1

        for sid, sdata in new_scenes.items():
            new_sid = f"scene{next_num:03d}"
            existing_scenes[new_sid] = sdata
            next_num += 1

        existing["scenes"] = existing_scenes
        # 如果 data 里有新的 title/pov/summary，也更新
        if data.get("title"):
            existing["title"] = data["title"]
        if data.get("pov"):
            existing["pov"] = data["pov"]
        if data.get("summary"):
            existing["summary"] = data["summary"]
        chapters[chapter_id] = existing

    outlines["chapters"] = chapters
    save_yaml(outlines_path, outlines)
    print(f"[完成] 已写入 {outlines_path}")


def generate_report(data: dict, chapter_id: str, idea: str, raw_output: str):
    """生成扩写报告"""
    root = get_project_root()

    lines = [
        f"# 大纲扩写报告: {chapter_id}\n",
        f"## 原始想法\n\n{idea}\n",
        f"## 生成结果\n",
        f"**标题**: {data.get('title', '')}",
        f"**POV**: {data.get('pov', '')}",
        f"**概要**: {data.get('summary', '')}\n",
        f"### 场景\n",
    ]

    scenes = data.get("scenes", {})
    for sid, sdata in scenes.items():
        lines.append(f"#### {sid}: {sdata.get('title', '')}")
        lines.append(f"- 目标: {sdata.get('goal', '')}")
        lines.append(f"- 地点: {sdata.get('location', '')}")
        lines.append(f"- 角色: {', '.join(sdata.get('characters', []))}")
        lines.append(f"- 冲突: {sdata.get('conflict', '')}")
        lines.append(f"- 钩子: {sdata.get('ending_hook', '')}")
        lines.append("")

    lines.append("## 写入后的 YAML 片段\n")
    lines.append("```yaml")
    lines.append(f"  {chapter_id}:")
    lines.append(f"    title: \"{data.get('title', '')}\"")
    lines.append(f"    pov: \"{data.get('pov', '')}\"")
    lines.append(f"    summary: \"{data.get('summary', '')}\"")
    lines.append(f"    scenes:")
    for sid, sdata in scenes.items():
        lines.append(f"      {sid}:")
        lines.append(f"        title: \"{sdata.get('title', '')}\"")
        lines.append(f"        goal: \"{sdata.get('goal', '')}\"")
        lines.append(f"        location: \"{sdata.get('location', '')}\"")
        lines.append(f"        characters: {json.dumps(sdata.get('characters', []), ensure_ascii=False)}")
        lines.append(f"        conflict: \"{sdata.get('conflict', '')}\"")
        lines.append(f"        ending_hook: \"{sdata.get('ending_hook', '')}\"")
    lines.append("```")

    lines.append("\n## Raw LLM Output (前 2000 字)\n")
    lines.append("```")
    lines.append(raw_output[:2000])
    lines.append("```")

    report_path = root / "outputs" / "reports" / f"{chapter_id}_outline_expansion_report.md"
    write_text(report_path, "\n".join(lines))
    print(f"[完成] 报告已保存: {report_path}")


def main():
    parser = argparse.ArgumentParser(
        description="将粗略的自然语言章节想法扩写成标准 chapter_outlines.yaml 结构",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
示例:
  python scripts/expand_chapter_outline.py --chapter ch001 --idea "第一章：王二买肉去看父亲"
  python scripts/expand_chapter_outline.py --chapter ch001 --idea-file inputs/ch001_idea.txt
  python scripts/expand_chapter_outline.py --chapter ch004 --idea "..." --num-scenes 4
  python scripts/expand_chapter_outline.py --chapter ch001 --idea "..." --overwrite
  python scripts/expand_chapter_outline.py --chapter ch001 --idea "..." --append-scenes
        """,
    )
    parser.add_argument("--chapter", required=True, help="章节 ID，如 ch001")

    idea_group = parser.add_mutually_exclusive_group(required=True)
    idea_group.add_argument("--idea", help="粗略的章节想法文本")
    idea_group.add_argument("--idea-file", help="从文件读取章节想法")

    parser.add_argument("--num-scenes", type=int, default=3, help="生成的场景数量（默认 3）")
    parser.add_argument("--overwrite", action="store_true", help="覆盖已有章节")
    parser.add_argument("--append-scenes", action="store_true", help="在已有章节后追加场景")
    args = parser.parse_args()

    # 互斥检查
    if args.overwrite and args.append_scenes:
        print("[错误] --overwrite 和 --append-scenes 不能同时使用")
        sys.exit(1)

    ensure_dirs()

    # 读取 idea
    if args.idea_file:
        idea_path = Path(args.idea_file)
        if not idea_path.exists():
            print(f"[错误] 文件不存在: {idea_path}")
            sys.exit(1)
        idea = read_text(idea_path).strip()
        if not idea:
            print(f"[错误] 文件为空: {idea_path}")
            sys.exit(1)
        print(f"[信息] 从文件读取想法: {idea_path}")
    else:
        idea = args.idea

    chapter_id = args.chapter

    # 检查已有章节
    root = get_project_root()
    outlines = load_yaml(root / "outlines" / "chapter_outlines.yaml")
    existing_chapters = outlines.get("chapters", {}) or {}
    chapter_exists = chapter_id in existing_chapters

    # 确定写入模式
    if args.overwrite:
        mode = "overwrite"
        print(f"[信息] 模式: 覆盖 {chapter_id}")
    elif args.append_scenes:
        if not chapter_exists:
            print(f"[警告] {chapter_id} 不存在，将新建章节")
            mode = "new"
        else:
            mode = "append"
            print(f"[信息] 模式: 追加场景到 {chapter_id}")
    elif chapter_exists:
        print(f"[错误] {chapter_id} 已存在。使用 --overwrite 覆盖或 --append-scenes 追加场景")
        sys.exit(1)
    else:
        mode = "new"

    print(f"[信息] 章节: {chapter_id}")
    print(f"[信息] 想法: {idea[:80]}{'...' if len(idea) > 80 else ''}")
    print(f"[信息] 场景数: {args.num_scenes}")

    # 调用 LLM 扩写
    data, raw_output = expand_chapter(chapter_id, idea, args.num_scenes)

    print(f"[成功] 扩写完成: {data.get('title', '?')} / {len(data.get('scenes', {}))} 个场景")

    # 写入 YAML
    write_to_chapter_outlines(data, chapter_id, mode)

    # 生成报告
    generate_report(data, chapter_id, idea, raw_output)

    print(f"\n[完成] {chapter_id} 大纲扩写全部完成！")


if __name__ == "__main__":
    main()
