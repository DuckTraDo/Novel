"""
reset_chapter.py
删除指定章节正文和报告。默认不删除长期记忆。

用法:
    python scripts/reset_chapter.py --chapter ch001
    python scripts/reset_chapter.py --chapter ch001 --include-memory
"""

import argparse
import json
import shutil
from pathlib import Path

import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))

from utils import get_project_root, ensure_dirs, read_jsonl, configure_utf8_stdio


def rewrite_jsonl(path: Path, records: list[dict]):
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        for record in records:
            f.write(json.dumps(record, ensure_ascii=False) + "\n")


def belongs_to_chapter(record: dict, chapter_id: str) -> bool:
    if record.get("chapter_id") == chapter_id:
        return True
    event_id = str(record.get("event_id", ""))
    return event_id.startswith(f"{chapter_id}_")


def filter_memory_records(chapter_id: str):
    root = get_project_root()
    targets = [
        root / "memory" / "chapter_summaries.jsonl",
        root / "memory" / "events.jsonl",
        root / "memory" / "timeline.jsonl",
    ]
    for path in targets:
        records = read_jsonl(path)
        kept = [record for record in records if not belongs_to_chapter(record, chapter_id)]
        removed = len(records) - len(kept)
        rewrite_jsonl(path, kept)
        print(f"[记忆] {path.name}: 删除 {removed} 条记录")


def main():
    configure_utf8_stdio()
    parser = argparse.ArgumentParser(description="删除某章正文和报告，默认不删除长期记忆")
    parser.add_argument("--chapter", required=True, help="章节 ID，如 ch001")
    parser.add_argument("--include-memory", action="store_true",
                        help="同时从 chapter_summaries/events/timeline 中过滤该章节记录")
    args = parser.parse_args()

    ensure_dirs()
    root = get_project_root()
    chapter_id = args.chapter.strip()

    chapter_dir = root / "chapters" / chapter_id
    if chapter_dir.exists():
        shutil.rmtree(chapter_dir)
        print(f"[删除] {chapter_dir}")
    else:
        print(f"[跳过] 章节目录不存在: {chapter_dir}")

    reports_dir = root / "outputs" / "reports"
    deleted_reports = 0
    for report in reports_dir.glob(f"{chapter_id}_*"):
        if report.is_file():
            report.unlink()
            deleted_reports += 1
            print(f"[删除] {report}")
    print(f"[完成] 已删除 {deleted_reports} 个报告文件")

    if args.include_memory:
        filter_memory_records(chapter_id)
    else:
        print("[信息] 未删除长期记忆。需要时可加 --include-memory。")


if __name__ == "__main__":
    main()
