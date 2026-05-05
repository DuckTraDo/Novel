"""
check_consistency.py
读取 chapters/<chapter_id>/chapter.md + memory，调用 LLM 检查一致性问题。

用法:
    python scripts/check_consistency.py --chapter ch001
"""

import argparse
import json
import re
import sys
from datetime import datetime
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from utils import (
    get_project_root, ensure_dirs, load_config,
    load_yaml, read_jsonl,
    read_text, write_text, call_local_llm,
    configure_utf8_stdio,
)


CONSISTENCY_CHECK_PROMPT = """你是一位经验丰富的小说编辑，专门负责长篇小说的连续性审查。

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

如果没有任何问题，请如实说明。"""


def load_chapter_idea(chapter_id: str) -> str:
    """从生成报告中读取作者 chapter idea；缺失时给出提示文本。"""
    report_path = get_project_root() / "outputs" / "reports" / f"{chapter_id}_chapter_generation_report.md"
    text = read_text(report_path)
    if not text:
        return "（未找到生成报告中的 chapter idea；如需检查此项，请参考作者原始输入。）"
    match = re.search(r"## Idea\s+(.*?)\s+## Target Words", text, re.DOTALL)
    if not match:
        return "（未能从生成报告解析 chapter idea；如需检查此项，请参考作者原始输入。）"
    return match.group(1).strip()


def build_check_data(chapter_id: str) -> dict:
    """收集检查所需的所有数据。"""
    root = get_project_root()
    chapter_path = root / "chapters" / chapter_id / "chapter.md"
    if not chapter_path.exists():
        print(f"[错误] 未找到 {chapter_path}")
        print("[提示] 请先运行: python scripts/generate_chapter_local.py --chapter "
              f"{chapter_id} --idea \"这一章大概发生什么。\"")
        sys.exit(1)

    chapter_text = read_text(chapter_path).strip()
    if not chapter_text:
        print(f"[错误] {chapter_path} 是空文件")
        sys.exit(1)

    characters = load_yaml(root / "memory" / "characters.yaml")
    story_bible = load_yaml(root / "memory" / "story_bible.yaml")
    foreshadowing = load_yaml(root / "memory" / "foreshadowing.yaml")
    events = read_jsonl(root / "memory" / "events.jsonl")
    timeline = read_jsonl(root / "memory" / "timeline.jsonl")

    return {
        "chapter_text": chapter_text,
        "chapter_idea": load_chapter_idea(chapter_id),
        "characters": characters,
        "story_bible": story_bible,
        "foreshadowing": foreshadowing,
        "recent_events": events[-20:],
        "recent_timeline": timeline[-20:],
    }


def run_consistency_check(chapter_id: str) -> str:
    """执行一致性检查，返回报告文本。"""
    data = build_check_data(chapter_id)

    characters_text = json.dumps(data["characters"], ensure_ascii=False, indent=2)
    world_rules = data["story_bible"].get("world_rules", [])
    writing_rules = data["story_bible"].get("writing_rules", [])
    forbidden = data["story_bible"].get("forbidden_patterns", [])
    world_rules_text = "\n".join([
        "世界观规则:", *[f"- {r}" for r in world_rules],
        "\n写作规则:", *[f"- {r}" for r in writing_rules],
        "\n禁止模式:", *[f"- {p}" for p in forbidden],
    ])
    foreshadowing_text = json.dumps(data["foreshadowing"], ensure_ascii=False, indent=2)
    events_text = "\n".join([
        f"- [{e.get('event_id', '?')}] {e.get('description', '')}"
        for e in data["recent_events"]
    ]) if data["recent_events"] else "（暂无事件记录）"
    timeline_text = "\n".join([
        f"- [{t.get('event_id', '?')}] {t.get('time', '')}: {t.get('description', '')}"
        for t in data["recent_timeline"]
    ]) if data["recent_timeline"] else "（暂无时间线记录）"

    prompt = CONSISTENCY_CHECK_PROMPT.format(
        chapter_id=chapter_id,
        chapter_text=data["chapter_text"],
        chapter_idea=data["chapter_idea"],
        characters_text=characters_text,
        world_rules_text=world_rules_text,
        foreshadowing_text=foreshadowing_text,
        events_text=events_text,
        timeline_text=timeline_text,
    )

    messages = [
        {"role": "system", "content": "你是一位专业的小说编辑，专注于连续性审查。请用中文输出报告。"},
        {"role": "user", "content": prompt},
    ]
    print("[信息] 正在调用模型进行一致性审查...")
    return call_local_llm(messages, temperature=0.4, top_p=0.9)


def main():
    configure_utf8_stdio()
    parser = argparse.ArgumentParser(description="检查完整章节一致性")
    parser.add_argument("--chapter", required=True, help="章节 ID，如 ch001")
    args = parser.parse_args()

    ensure_dirs()
    chapter_id = args.chapter
    print(f"[信息] 正在对 {chapter_id} 进行一致性审查...")

    report = run_consistency_check(chapter_id)

    config = load_config()
    header = f"""<!-- 一致性审查报告 -->
<!-- 生成时间: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')} -->
<!-- 模型: {config.get('model_name', '未知')} -->

"""
    root = get_project_root()
    report_path = root / "outputs" / "reports" / f"{chapter_id}_consistency_report.md"
    write_text(report_path, header + report)
    print(f"[完成] 一致性审查报告已保存: {report_path}")


if __name__ == "__main__":
    main()
