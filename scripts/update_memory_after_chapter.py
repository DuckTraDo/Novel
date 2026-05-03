"""
update_memory_after_chapter.py
章节写完后，读取某章所有 scene，调用 LLM 抽取结构化记忆更新。

用法:
    python scripts/update_memory_after_chapter.py --chapter ch001
"""

import argparse
import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from utils import (
    get_project_root, ensure_dirs, load_yaml, save_yaml,
    load_json, save_json, append_jsonl, find_scene_files,
    read_text, write_text, call_local_llm,
)


EXTRACTION_PROMPT_TEMPLATE = """你是一位专业的小说编辑助手。请阅读以下章节正文，然后以严格的JSON格式输出结构化的记忆更新信息。

## 章节正文

{chapter_text}

## 现有角色档案

{characters_text}

## 现有伏笔

{foreshadowing_text}

## 输出要求

请输出一个严格的JSON对象，不要包含任何其他文字、解释或markdown标记。JSON schema 如下：

{{
  "chapter_id": "{chapter_id}",
  "chapter_summary": "用2-3句话概括本章发生了什么",
  "events": [
    {{
      "event_id": "{chapter_id}_evt001",
      "type": "scene/conflict/revelation/decision/travel/battle",
      "characters": ["角色名"],
      "description": "事件描述",
      "location": "地点",
      "time_order": "相对时间描述",
      "importance": 1到10的整数
    }}
  ],
  "character_updates": [
    {{
      "name": "角色名",
      "field": "current_status/knows/secrets/relationships/constraints",
      "old_value": "旧值（如果知道的话）",
      "new_value": "新值",
      "reason": "变化原因"
    }}
  ],
  "foreshadowing_updates": [
    {{
      "id": "伏笔ID",
      "action": "add/resolve/update",
      "description": "描述",
      "related_characters": ["角色名"],
      "planned_resolution": "计划回收方式",
      "status": "active/resolved"
    }}
  ],
  "relationship_updates": [
    {{
      "source": "角色A",
      "target": "角色B",
      "relation": "关系描述",
      "change": "变化说明"
    }}
  ],
  "continuity_risks": [
    {{
      "risk": "风险描述",
      "severity": "low/medium/high",
      "suggestion": "建议"
    }}
  ]
}}"""


def extract_chapter_text(chapter_id: str) -> str:
    """读取某章所有场景文件，拼接成完整文本"""
    scene_files = find_scene_files(chapter_id)
    if not scene_files:
        print(f"[错误] 未找到 {chapter_id} 的场景文件")
        sys.exit(1)
    parts = []
    for sf in scene_files:
        text = read_text(sf)
        if text.strip():
            parts.append(f"--- {sf.stem} ---\n{text}")
    return "\n\n".join(parts)


def call_llm_for_extraction(chapter_id: str, chapter_text: str) -> str:
    """调用 LLM 进行记忆抽取"""
    root = get_project_root()
    characters = load_yaml(root / "memory" / "characters.yaml")
    foreshadowing = load_yaml(root / "memory" / "foreshadowing.yaml")

    characters_text = json.dumps(characters, ensure_ascii=False, indent=2)
    foreshadowing_text = json.dumps(foreshadowing, ensure_ascii=False, indent=2)

    prompt = EXTRACTION_PROMPT_TEMPLATE.format(
        chapter_id=chapter_id,
        chapter_text=chapter_text,
        characters_text=characters_text,
        foreshadowing_text=foreshadowing_text,
    )

    messages = [
        {"role": "system", "content": "你只输出严格合法的JSON，不输出任何其他文本。"},
        {"role": "user", "content": prompt},
    ]
    return call_local_llm(messages, temperature=0.3, top_p=0.85)


def parse_extraction_json(raw_output: str) -> dict:
    """尝试从模型输出中提取 JSON"""
    text = raw_output.strip()
    json_match = re.search(r'```(?:json)?\s*\n?(.*?)\n?```', text, re.DOTALL)
    if json_match:
        text = json_match.group(1).strip()
    if text and text[0] not in ('{', '['):
        start = text.find('{')
        if start != -1:
            text = text[start:]
    last_brace = text.rfind('}')
    if last_brace != -1:
        text = text[:last_brace + 1]
    try:
        return json.loads(text)
    except json.JSONDecodeError as e:
        raise ValueError(f"JSON解析失败: {e}\n原始输出前500字:\n{raw_output[:500]}")


def apply_memory_updates(data: dict, chapter_id: str):
    """将抽取结果应用到 memory 文件"""
    root = get_project_root()

    # 1. 追加章节摘要
    append_jsonl(root / "memory" / "chapter_summaries.jsonl", {
        "chapter_id": chapter_id,
        "chapter_summary": data.get("chapter_summary", ""),
    })
    print(f"[更新] 已追加章节摘要")

    # 2. 追加事件
    events = data.get("events", [])
    for evt in events:
        append_jsonl(root / "memory" / "events.jsonl", evt)
    if events:
        print(f"[更新] 已追加 {len(events)} 条事件")

    # 3. 追加 timeline
    for evt in events:
        append_jsonl(root / "memory" / "timeline.jsonl", {
            "event_id": evt.get("event_id", ""),
            "time": evt.get("time_order", ""),
            "description": evt.get("description", ""),
            "characters": evt.get("characters", []),
            "importance": evt.get("importance", 5),
        })
    if events:
        print(f"[更新] 已追加 {len(events)} 条时间线记录")

    # 4. 更新角色档案
    if data.get("character_updates"):
        update_characters(data["character_updates"])

    # 5. 更新伏笔
    if data.get("foreshadowing_updates"):
        update_foreshadowing(data["foreshadowing_updates"])

    # 6. 更新关系图
    if data.get("relationship_updates"):
        update_relationships(data["relationship_updates"])


def update_characters(updates: list):
    """保守地更新角色档案"""
    root = get_project_root()
    chars_path = root / "memory" / "characters.yaml"
    chars_data = load_yaml(chars_path)
    characters = chars_data.get("characters", [])
    char_map = {c["name"]: i for i, c in enumerate(characters)}

    for update in updates:
        name = update.get("name", "")
        field = update.get("field", "")
        new_value = update.get("new_value", "")
        if name not in char_map:
            print(f"[警告] 角色 '{name}' 不存在，跳过")
            continue
        idx = char_map[name]
        char = characters[idx]

        if field == "current_status":
            char["current_status"] = new_value
            print(f"[更新] {name}.current_status -> {new_value[:50]}...")
        elif field in ("knows", "secrets", "constraints"):
            current_list = char.get(field, [])
            if isinstance(current_list, list):
                if new_value not in current_list:
                    current_list.append(new_value)
                    char[field] = current_list
                    print(f"[更新] {name}.{field} += {new_value[:50]}...")
            else:
                char[field] = [new_value]
        elif field == "relationships":
            current_rels = char.get("relationships", [])
            if isinstance(current_rels, list):
                if new_value not in current_rels:
                    current_rels.append(new_value)
                    char["relationships"] = current_rels
                    print(f"[更新] {name}.relationships += {new_value[:50]}...")

    chars_data["characters"] = characters
    save_yaml(chars_path, chars_data)
    print(f"[完成] 角色档案已更新")


def update_foreshadowing(updates: list):
    """更新伏笔追踪"""
    root = get_project_root()
    fs_path = root / "memory" / "foreshadowing.yaml"
    fs_data = load_yaml(fs_path)
    active = fs_data.get("active", [])
    resolved = fs_data.get("resolved", [])
    active_map = {fs["id"]: i for i, fs in enumerate(active)}

    for update in updates:
        action = update.get("action", "")
        fs_id = update.get("id", "")
        if action == "add":
            active.append({
                "id": fs_id,
                "introduced_in": update.get("introduced_in", ""),
                "description": update.get("description", ""),
                "related_characters": update.get("related_characters", []),
                "planned_resolution": update.get("planned_resolution", ""),
                "status": "active",
            })
            print(f"[伏笔] 新增: {fs_id}")
        elif action == "resolve":
            if fs_id in active_map:
                fs = active.pop(active_map[fs_id])
                fs["status"] = "resolved"
                resolved.append(fs)
                print(f"[伏笔] 回收: {fs_id}")
        elif action == "update":
            if fs_id in active_map:
                idx = active_map[fs_id]
                if update.get("description"):
                    active[idx]["description"] = update["description"]
                if update.get("planned_resolution"):
                    active[idx]["planned_resolution"] = update["planned_resolution"]
                print(f"[伏笔] 更新: {fs_id}")

    fs_data["active"] = active
    fs_data["resolved"] = resolved
    save_yaml(fs_path, fs_data)
    print(f"[完成] 伏笔追踪已更新")


def update_relationships(updates: list):
    """更新关系图"""
    root = get_project_root()
    rel_path = root / "memory" / "relationships.json"
    rel_data = load_json(rel_path)
    nodes = rel_data.get("nodes", [])
    edges = rel_data.get("edges", [])
    node_ids = {n["id"] for n in nodes}
    edge_keys = {(e["source"], e["target"]) for e in edges}

    for update in updates:
        source = update.get("source", "")
        target = update.get("target", "")
        relation = update.get("relation", "")
        if source not in node_ids:
            nodes.append({"id": source, "type": "character", "role": ""})
            node_ids.add(source)
        if target not in node_ids:
            nodes.append({"id": target, "type": "character", "role": ""})
            node_ids.add(target)
        edge_key = (source, target)
        if edge_key in edge_keys:
            for e in edges:
                if e["source"] == source and e["target"] == target:
                    e["relation"] = relation
                    if update.get("change"):
                        e["change"] = update["change"]
                    break
        else:
            edges.append({"source": source, "target": target, "relation": relation, "change": update.get("change", "")})
            edge_keys.add(edge_key)
        print(f"[关系] {source} --[{relation}]--> {target}")

    rel_data["nodes"] = nodes
    rel_data["edges"] = edges
    save_json(rel_path, rel_data)
    print(f"[完成] 关系图已更新")


def generate_report(data: dict, chapter_id: str, raw_output: str):
    """生成记忆更新报告"""
    root = get_project_root()
    lines = [
        f"# 记忆更新报告: {chapter_id}\n",
        f"## 章节摘要\n\n{data.get('chapter_summary', '无')}\n",
        f"## 新增事件 ({len(data.get('events', []))})\n",
    ]
    for evt in data.get("events", []):
        lines.append(f"- **[{evt.get('event_id', '?')}]** {evt.get('description', '')}")
        lines.append(f"  - 类型: {evt.get('type', '')} | 角色: {', '.join(evt.get('characters', []))} | 地点: {evt.get('location', '')}\n")

    lines.append(f"## 角色更新 ({len(data.get('character_updates', []))})\n")
    for cu in data.get("character_updates", []):
        lines.append(f"- **{cu.get('name', '?')}.{cu.get('field', '?')}**: {cu.get('new_value', '')}")
        lines.append(f"  - 原因: {cu.get('reason', '')}\n")

    lines.append(f"## 伏笔更新 ({len(data.get('foreshadowing_updates', []))})\n")
    for fu in data.get("foreshadowing_updates", []):
        lines.append(f"- **[{fu.get('id', '?')}]** {fu.get('action', '')}: {fu.get('description', '')}\n")

    lines.append(f"## 关系更新 ({len(data.get('relationship_updates', []))})\n")
    for ru in data.get("relationship_updates", []):
        lines.append(f"- {ru.get('source', '?')} --[{ru.get('relation', '?')}]--> {ru.get('target', '?')}\n")

    lines.append(f"## 连续性风险 ({len(data.get('continuity_risks', []))})\n")
    for risk in data.get("continuity_risks", []):
        severity = risk.get("severity", "low")
        emoji = {"low": "低", "medium": "中", "high": "高"}.get(severity, "?")
        lines.append(f"- [{emoji}] {risk.get('risk', '')}")
        lines.append(f"  - 建议: {risk.get('suggestion', '')}\n")

    lines.append("## Raw LLM Output (前2000字)\n\n```")
    lines.append(raw_output[:2000])
    lines.append("```")

    report_path = root / "outputs" / "reports" / f"{chapter_id}_memory_update_report.md"
    write_text(report_path, "\n".join(lines))
    print(f"[完成] 更新报告已保存: {report_path}")


def main():
    parser = argparse.ArgumentParser(description="章节完成后更新记忆")
    parser.add_argument("--chapter", required=True, help="章节 ID，如 ch001")
    args = parser.parse_args()

    ensure_dirs()
    chapter_id = args.chapter

    print(f"[信息] 正在读取 {chapter_id} 的所有场景...")
    chapter_text = extract_chapter_text(chapter_id)
    print(f"[信息] 章节文本长度: {len(chapter_text)} 字符")

    print(f"[信息] 正在调用模型进行记忆抽取...")
    raw_output = call_llm_for_extraction(chapter_id, chapter_text)

    try:
        data = parse_extraction_json(raw_output)
        print(f"[成功] JSON 解析成功")
    except ValueError as e:
        print(f"[错误] {e}")
        fail_path = get_project_root() / "outputs" / "reports" / f"{chapter_id}_memory_parse_failed.md"
        write_text(fail_path, f"# 记忆抽取 JSON 解析失败\n\n{raw_output}")
        print(f"[信息] 原始输出已保存到: {fail_path}")
        sys.exit(1)

    print(f"[信息] 正在应用记忆更新...")
    apply_memory_updates(data, chapter_id)
    generate_report(data, chapter_id, raw_output)
    print(f"\n[完成] {chapter_id} 的记忆更新全部完成！")


if __name__ == "__main__":
    main()
