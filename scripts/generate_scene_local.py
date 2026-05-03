"""
generate_scene_local.py
调用本地模型，根据 build_context 生成场景正文。

用法:
    python scripts/generate_scene_local.py --chapter ch001 --scene scene001
"""

import argparse
import sys
from pathlib import Path
from datetime import datetime

sys.path.insert(0, str(Path(__file__).resolve().parent))

from utils import (
    get_project_root, ensure_dirs, load_config,
    read_text, write_text, call_local_llm, simple_token_estimate,
)


SYSTEM_PROMPT = """你是一位专业的中文长篇小说作者。你的任务是根据提供的上下文信息，写出高质量的小说场景正文。

核心原则：
- 只输出小说正文，不要输出任何解释、分析、标题或总结
- 不要以"以下是"、"下面是"、"根据上下文"等词开头
- 不要在末尾加总结段落或道德说教
- 保持人物语气与角色档案中描述的一致
- 遵守世界观规则，不要引入矛盾设定
- 如果上下文没有明确事实，可以合理发挥，但不要编造会破坏已有设定的大事件
- 注意段落节奏，长短交替，避免所有段落一样长
- 对话要有潜台词，不要直白地交换信息
- 场景结尾要留钩子

禁止出现的AI腔：
- "首先...其次...最后..."
- "一方面...另一方面..."
- "他心想"
- "她感到一阵莫名的"
- "命运的齿轮开始转动"
- "不知不觉"
- "突然"作为段落开头"""


def build_generation_prompt(context: str) -> list:
    """构建发送给 LLM 的消息列表"""
    user_prompt = f"""以下是当前场景的完整写作上下文，请据此写出场景正文：

{context}

---

请开始写作。只输出正文内容。"""
    return [
        {"role": "system", "content": SYSTEM_PROMPT},
        {"role": "user", "content": user_prompt},
    ]


def generate_scene(chapter_id: str, scene_id: str) -> tuple:
    """
    生成场景正文。
    返回: (scene_text, raw_response)
    """
    root = get_project_root()

    # 确保 context 文件存在
    context_path = root / "outputs" / "contexts" / f"{chapter_id}_{scene_id}_context.md"
    if not context_path.exists():
        print(f"[信息] 上下文文件不存在，正在生成...")
        from build_context import build_context
        context_text = build_context(chapter_id, scene_id)
        write_text(context_path, context_text)
        print(f"[信息] 上下文已生成: {context_path}")
    else:
        context_text = read_text(context_path)

    # 估算 token
    config = load_config()
    context_tokens = simple_token_estimate(context_text)
    max_context = config.get("max_context_tokens", 30000)
    print(f"[信息] 上下文 token 估算: ~{context_tokens} / {max_context}")

    # 构建 prompt 并调用 LLM
    messages = build_generation_prompt(context_text)
    print(f"[信息] 正在调用本地模型生成 {chapter_id}/{scene_id}...")
    raw_response = call_local_llm(messages)

    scene_text = clean_output(raw_response)
    return scene_text, raw_response


def clean_output(raw: str) -> str:
    """清理模型输出，去掉常见的 AI 腔开头和结尾"""
    text = raw.strip()
    prefixes_to_remove = [
        "以下是场景正文：", "以下是正文：", "下面是场景正文：",
        "下面是正文：", "根据上下文，", "好的，", "好的。",
    ]
    for prefix in prefixes_to_remove:
        if text.startswith(prefix):
            text = text[len(prefix):].strip()
    if text.startswith("```"):
        first_newline = text.find("\n")
        if first_newline != -1:
            text = text[first_newline + 1:]
        if text.rstrip().endswith("```"):
            text = text.rstrip()[:-3].rstrip()
    return text


def main():
    parser = argparse.ArgumentParser(description="调用本地模型生成场景正文")
    parser.add_argument("--chapter", required=True, help="章节 ID，如 ch001")
    parser.add_argument("--scene", required=True, help="场景 ID，如 scene001")
    args = parser.parse_args()

    ensure_dirs()
    print(f"[信息] 开始生成 {args.chapter}/{args.scene}...")

    scene_text, raw_response = generate_scene(args.chapter, args.scene)

    root = get_project_root()

    # 保存场景正文
    scene_dir = root / "chapters" / args.chapter
    scene_dir.mkdir(parents=True, exist_ok=True)
    scene_path = scene_dir / f"{args.scene}.md"
    write_text(scene_path, scene_text)
    print(f"[完成] 场景正文已保存: {scene_path}")

    # 保存生成报告
    config = load_config()
    report_lines = [
        f"# 生成报告: {args.chapter}/{args.scene}",
        f"",
        f"**生成时间**: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}",
        f"**模型**: {config.get('model_name', '未知')}",
        f"**温度**: {config.get('generation_temperature', 'N/A')}",
        f"**top_p**: {config.get('generation_top_p', 'N/A')}",
        f"",
        f"## 正文长度",
        f"",
        f"- 字符数: {len(scene_text)}",
        f"- 估算 token: ~{simple_token_estimate(scene_text)}",
        f"",
        f"## Raw Response",
        f"",
        f"```",
        raw_response,
        f"```",
    ]
    report_path = root / "outputs" / "reports" / f"{args.chapter}_{args.scene}_generation_report.md"
    write_text(report_path, "\n".join(report_lines))
    print(f"[完成] 生成报告已保存: {report_path}")


if __name__ == "__main__":
    main()
