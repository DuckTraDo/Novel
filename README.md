# 长篇小说本地模型写作 Pipeline

用本地 OpenAI-compatible API（如 Qwen）写长篇小说的自动化 pipeline。

由于模型上下文窗口有限（32K），本 pipeline 实现场景级 context builder + 章节后记忆更新，
让模型在每一步都能获得最相关的上下文，而不需要把整本小说塞进去。

## 项目结构

```
pipeline/
├── config.yaml                 # 全局配置
├── README.md
│
├── memory/                     # 持久化记忆系统
│   ├── story_bible.yaml        # 世界观和写作规则
│   ├── characters.yaml         # 角色档案
│   ├── foreshadowing.yaml      # 伏笔追踪
│   ├── relationships.json      # 人物关系图
│   ├── timeline.jsonl          # 时间线
│   ├── events.jsonl            # 事件记录
│   ├── chapter_summaries.jsonl # 章节摘要
│   └── style_bank.jsonl        # 风格参考库
│
├── outlines/                   # 大纲
│   ├── book_outline.yaml       # 全书大纲
│   └── chapter_outlines.yaml   # 章节大纲
│
├── chapters/                   # 生成的小说正文
│   └── ch001/
│       ├── scene001.md
│       └── scene002.md
│
├── outputs/                    # 输出
│   ├── contexts/               # 生成的上下文文件
│   └── reports/                # 各种报告
│
└── scripts/                    # 脚本
    ├── utils.py                # 通用工具函数
    ├── expand_chapter_outline.py  # 章节大纲扩写
    ├── build_context.py        # 构建场景上下文
    ├── generate_scene_local.py # 调用本地模型生成场景
    ├── update_memory_after_chapter.py  # 章节后记忆更新
    └── check_consistency.py    # 一致性检查
```

## 环境准备

### 安装依赖

```powershell
pip install openai pyyaml
```

### 设置环境变量（PowerShell）

```powershell
$env:LLM_BASE_URL="http://localhost:18083/v1"
$env:LLM_API_KEY="local"
$env:LLM_MODEL="your-model-name.gguf"
```

如果不设置 `LLM_MODEL`，将使用 `config.yaml` 中的 `model_name`。

## 使用流程

### 1. 编辑大纲和角色

开始写作前，先编辑：

- `memory/story_bible.yaml` — 世界观、写作规则、禁止模式
- `memory/characters.yaml` — 角色档案
- `outlines/book_outline.yaml` — 全书大纲

### 2. 扩写章节大纲

用一句话描述章节想法，自动扩写成结构化大纲：

```powershell
python scripts/expand_chapter_outline.py --chapter ch001 --idea "第一章：王二在肉铺下班，买半斤肉去桥下看父亲。父子关系冷淡，父亲咳血但藏起来。"
```

也支持从文件读取：

```powershell
python scripts/expand_chapter_outline.py --chapter ch001 --idea-file inputs/ch001_idea.txt
```

可选参数：
- `--num-scenes 4` — 生成 4 个场景（默认 3）
- `--overwrite` — 覆盖已有章节
- `--append-scenes` — 在已有章节后追加场景

### 3. 生成场景上下文

```powershell
python scripts/build_context.py --chapter ch001 --scene scene001
```

读取所有 memory 和 outline 文件，生成当前场景的上下文，保存到 `outputs/contexts/ch001_scene001_context.md`。

### 4. 生成场景正文

```powershell
python scripts/generate_scene_local.py --chapter ch001 --scene scene001
```

如果 context 文件不存在会自动先生成。生成正文保存到 `chapters/ch001/scene001.md`，报告保存到 `outputs/reports/`。

### 5. 章节完成后更新记忆

```powershell
python scripts/update_memory_after_chapter.py --chapter ch001
```

读取该章所有 scene，调用 LLM 抽取结构化记忆，更新所有 memory 文件。

### 6. 一致性检查（可选）

```powershell
python scripts/check_consistency.py --chapter ch001
```

检查人物语气、世界观冲突、时间线矛盾、伏笔问题等，生成报告到 `outputs/reports/`。

## 完整写作循环

```powershell
# 第一步：编辑大纲和角色（手动）
# 第二步：扩写章节大纲
python scripts/expand_chapter_outline.py --chapter ch001 --idea "第一章：王二在肉铺下班，买半斤肉去桥下看父亲。"

# 第三步：生成场景
python scripts/build_context.py --chapter ch001 --scene scene001
python scripts/generate_scene_local.py --chapter ch001 --scene scene001

# 重复 scene002, scene003...
python scripts/build_context.py --chapter ch001 --scene scene002
python scripts/generate_scene_local.py --chapter ch001 --scene scene002

# 第四步：章节完成后更新记忆
python scripts/update_memory_after_chapter.py --chapter ch001

# 第五步：一致性检查
python scripts/check_consistency.py --chapter ch001

# 然后进入下一章...
```

## 配置说明

编辑 `config.yaml` 可调整：

- `model_name` — 模型名称（环境变量 `LLM_MODEL` 优先）
- `max_context_tokens` — 上下文窗口大小
- `generation_temperature` — 生成温度
- `generation_top_p` — top_p 采样
- `max_output_tokens` — 最大输出 token 数
- `default_style_rules` — 默认风格规则

## 记忆系统说明

- **story_bible.yaml** — 全局世界观和写作规则，所有场景共享
- **characters.yaml** — 角色档案，每章后自动更新（保守策略：追加不删除）
- **foreshadowing.yaml** — 伏笔追踪，自动跟踪新增/回收/更新
- **relationships.json** — 人物关系图，自动扩展
- **timeline.jsonl** — 时间线记录
- **events.jsonl** — 事件记录
- **chapter_summaries.jsonl** — 章节摘要（context 中取最近 5 条）
- **style_bank.jsonl** — 风格参考库（context 中取前 3 条）

## 注意事项

- 所有脚本从 `pipeline/` 根目录运行
- Windows 路径兼容
- 所有输出用 UTF-8 编码
- Token 估算用 `len(text) // 2` 粗略近似
- 第一版用简单规则选上下文，不做复杂 RAG
