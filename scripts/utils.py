"""
工具函数模块
提供文件读写、配置加载、LLM调用等通用功能
"""

import os
import sys
import json
import yaml
from pathlib import Path
from typing import Any, Optional


# ============================================================
# 路径工具
# ============================================================

def get_project_root() -> Path:
    """
    获取项目根目录（pipeline/）
    从当前文件位置向上查找
    """
    return Path(__file__).resolve().parent.parent


def ensure_dirs():
    """确保所有必要的目录存在"""
    root = get_project_root()
    dirs = [
        root / "memory",
        root / "outlines",
        root / "chapters",
        root / "outputs",
        root / "outputs" / "contexts",
        root / "outputs" / "reports",
    ]
    for d in dirs:
        d.mkdir(parents=True, exist_ok=True)


# ============================================================
# YAML 工具
# ============================================================

def load_yaml(path: Path) -> Any:
    """加载 YAML 文件，不存在则返回空字典"""
    if not path.exists():
        return {}
    with open(path, "r", encoding="utf-8") as f:
        return yaml.safe_load(f) or {}


def save_yaml(path: Path, data: Any):
    """保存数据到 YAML 文件"""
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        yaml.dump(data, f, allow_unicode=True, default_flow_style=False, sort_keys=False)


# ============================================================
# JSON 工具
# ============================================================

def load_json(path: Path) -> Any:
    """加载 JSON 文件，不存在则返回空字典"""
    if not path.exists():
        return {}
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def save_json(path: Path, data: Any):
    """保存数据到 JSON 文件"""
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)


# ============================================================
# JSONL 工具
# ============================================================

def append_jsonl(path: Path, obj: dict):
    """向 JSONL 文件追加一条记录"""
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "a", encoding="utf-8") as f:
        f.write(json.dumps(obj, ensure_ascii=False) + "\n")


def read_jsonl(path: Path) -> list:
    """读取 JSONL 文件的所有记录"""
    if not path.exists():
        return []
    records = []
    with open(path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line and line != "{}":
                try:
                    records.append(json.loads(line))
                except json.JSONDecodeError:
                    continue
    return records


# ============================================================
# 文本文件工具
# ============================================================

def read_text(path: Path) -> str:
    """读取文本文件"""
    if not path.exists():
        return ""
    with open(path, "r", encoding="utf-8") as f:
        return f.read()


def write_text(path: Path, text: str):
    """写入文本文件"""
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)


# ============================================================
# Token 估算
# ============================================================

def simple_token_estimate(text: str) -> int:
    """
    粗略估算中文文本的 token 数
    不用 tiktoken，用 len(text) / 2 作为近似值
    """
    if not text:
        return 0
    return max(1, len(text) // 2)


# ============================================================
# 配置加载
# ============================================================

def load_config() -> dict:
    """加载 config.yaml"""
    root = get_project_root()
    config_path = root / "config.yaml"
    config = load_yaml(config_path)
    if not config:
        print("[警告] config.yaml 为空或不存在，使用默认配置")
    return config


# ============================================================
# LLM 调用
# ============================================================

def get_llm_config() -> dict:
    """
    获取 LLM 配置
    优先使用环境变量，fallback 到 config.yaml
    返回: {"base_url": str, "api_key": str, "model": str}
    """
    config = load_config()
    base_url = os.environ.get("LLM_BASE_URL", "http://localhost:18083/v1")
    api_key = os.environ.get("LLM_API_KEY", "local")
    model = os.environ.get("LLM_MODEL") or config.get("model_name", "default-model")
    return {"base_url": base_url, "api_key": api_key, "model": model}


def call_local_llm(
    messages: list,
    temperature: Optional[float] = None,
    top_p: Optional[float] = None,
    max_tokens: Optional[int] = None,
) -> str:
    """
    调用本地 OpenAI-compatible API

    Args:
        messages: OpenAI 格式的消息列表 [{"role": ..., "content": ...}]
        temperature: 温度参数
        top_p: top_p 参数
        max_tokens: 最大输出 token 数

    Returns:
        模型输出的文本
    """
    from openai import OpenAI

    llm_cfg = get_llm_config()
    config = load_config()

    client = OpenAI(
        base_url=llm_cfg["base_url"],
        api_key=llm_cfg["api_key"],
    )

    if temperature is None:
        temperature = config.get("generation_temperature", 0.8)
    if top_p is None:
        top_p = config.get("generation_top_p", 0.9)
    if max_tokens is None:
        max_tokens = config.get("max_output_tokens", 4000)

    response = client.chat.completions.create(
        model=llm_cfg["model"],
        messages=messages,
        temperature=temperature,
        top_p=top_p,
        max_tokens=max_tokens,
    )

    return response.choices[0].message.content


def call_local_llm_raw(prompt: str, system_prompt: str = "", **kwargs) -> str:
    """简化的 LLM 调用接口"""
    messages = []
    if system_prompt:
        messages.append({"role": "system", "content": system_prompt})
    messages.append({"role": "user", "content": prompt})
    return call_local_llm(messages, **kwargs)


# ============================================================
# 文件发现工具
# ============================================================

def find_scene_files(chapter_id: str) -> list:
    """查找某章节下所有已写好的场景文件，按场景编号排序"""
    root = get_project_root()
    chapter_dir = root / "chapters" / chapter_id
    if not chapter_dir.exists():
        return []
    scenes = sorted(chapter_dir.glob("scene*.md"))
    return scenes


def get_scene_ending(chapter_id: str, scene_id: str, char_limit: int = 1200) -> str:
    """获取当前场景之前的上一个场景的末尾文本（最后 char_limit 个字符）"""
    scene_files = find_scene_files(chapter_id)
    prev_file = None
    for sf in scene_files:
        if sf.stem < scene_id:
            prev_file = sf
        else:
            break
    if prev_file is None:
        return ""
    text = read_text(prev_file)
    if len(text) > char_limit:
        return "..." + text[-char_limit:]
    return text


if __name__ == "__main__":
    root = get_project_root()
    print(f"项目根目录: {root}")
    ensure_dirs()
    print("目录结构已确认")
    config = load_config()
    print(f"配置加载成功: model_name = {config.get('model_name', 'N/A')}")
    llm_cfg = get_llm_config()
    print(f"LLM 配置: {llm_cfg}")
