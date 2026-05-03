"""
check_consistency.py
读取某章正文 + memory，调用 LLM 检查一致性问题。

用法:
    python scripts/check_consistency.py --chapter ch001
"""

import argparse
import json
import sys
from pathlib import Path
from datetime import datetime

sys.path.insert(0, str(Path(__file__).resolve().parent))

from utils import (
    get_project_root, ensure_dirs, load_config,
    load_yaml, read_jsonl, find_scene_files,
    read_text, write_text, call_local_llm,
)


CONSISTENCY_CHECK_PROMPT = """你是一位经验丰富的小说编辑，专门负责长篇小说的连续性审查。

请仔细阅读以下章节正文和相关记忆档案，找出所有一致性问题。

## 检查清单

1. **人物语气一致性**: 角色的说话方式是否与角色档案中描述的一致？
2. **信息超前**: 角色是否知道了他们不应该知道的信息？（参考 knows 和 secrets 字段）
3. **世界观冲突**: 正文中是否违反了 Story Bible 中的世界观规则？
4. **时间线矛盾**: 事件的时间顺序是否有矛盾？
5. **伏笔问题**: 已铺设的伏笔是否被意外揭示或遗忘？是否有冲突？
6. **无铺垫设定**: 是否出现了前文没有铺垫的重大新设定？
7. **AI 腔检查**: 是否有明显的 AI 生成痕迹？（总结腔、解释腔、列举句式等）

## 章节正文

{chapter_text}

## 角色档案

{characters_text}

## 世界观规则

{world_rules_text}

## 活跃伏笔

{foreshadowing_text}

## 最近事件

{events_text}

## 输出要求

请输出一份 Markdown 格式的审查报告。格式如下：

# 一致性审查报告: {chapter_id}

## 总体评价
（一段话概括质量）

## 发现的问题

### 严重问题
- **问题**: （描述）
  - **位置**: （大约在哪段/哪个场景）
  - **原因**: （为什么是问题）
  - **建议**: （如何修复）

### 中等问题
- ...

### 轻微问题/建议
- ...

## 亮点
（如果有的话，指出写得好的地方）

如果没有任何问题，请如实说明。"""


def build_check_data(chapter_id: str) -> dict:
    """收集检查所需的所有数据"""
    root = get_project_root()
    scene_files = find_scene_files(chapter_id)
    if not scene_files:
        print(f"[错误] 未找到 {chapter_id} 的场景文件")
        sys.exit(1)

    chapter_parts = []
    for sf in scene_files:
        text = read_text(sf)
        if text.strip():
            chapter_parts.append(f"### {sf.stem}\n\n{text}")
    chapter_text = "\n\n---\n\n".join(chapter_parts)

    characters = load_yaml(root / "memory" / "characters.yaml")
    story_bible = load_yaml(root / "memory" / "story_bible.yaml")
    foreshadowing = load_yaml(root / "memory" / "foreshadowing.yaml")
    events = read_jsonl(root / "memory" / "events.jsonl")

    return {
        "chapter_text": chapter_text,
        "characters": characters,
        "story_bible": story_bible,
        "foreshadowing": foreshadowing,
        "recent_events": events[-20:],
    }


def run_consistency_check(chapter_id: str) -> str:
    """执行一致性检查，返回报告文本"""
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

    prompt = CONSISTENCY_CHECK_PROMPT.format(
        chapter_id=chapter_id,
        chapter_text=data["chapter_text"],
        characters_text=characters_text,
        world_rules_text=world_rules_text,
        foreshadowing_text=foreshadowing_text,
        events_text=events_text,
    )

    messages = [
        {"role": "system", "content": "你是一位专业的小说编辑，专注于连续性审查。请用中文输出报告。"},
        {"role": "user", "content": prompt},
    ]
    print(f"[信息] 正在调用模型进行一致性审查...")
    return call_local_llm(messages, temperature=0.4, top_p=0.9)


def main():
    parser = argparse.ArgumentParser(description="检查章节一致性")
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
