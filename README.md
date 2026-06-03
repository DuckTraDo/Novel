<div align="center">

<img src="app/src-tauri/icons/128x128.png" width="96" alt="Sodarie Novel" />

# Sodarie Novel

**本地大模型 · 结构化记忆 · 长篇小说写作台**

一台「你掌舵、模型划桨」的长篇小说创作机器：你只用一句话定方向，模型一次写出整章，
流水线负责长期记忆、连续性检查和上下文组织——越写越连贯，而不是越写越乱。

[![Desktop App](https://img.shields.io/badge/Desktop-Tauri%20App-6d5efc)](#-sodarie-novel-桌面应用)
[![Rust](https://img.shields.io/badge/Engine-Rust-DEA584?logo=rust&logoColor=white)](#)
[![Local LLM](https://img.shields.io/badge/Local-LLM-2E7D32)](#)
[![OpenAI Compatible](https://img.shields.io/badge/OpenAI-Compatible-111827)](#)
[![Subscription LLM](https://img.shields.io/badge/Claude%20Code%20%2F%20Codex-订阅-8b5cf6)](#-三种-llm-连接方式)
[![i18n](https://img.shields.io/badge/中文-%2F%20English-0ea5e9)](#)
[![LoRA](https://img.shields.io/badge/Hugging%20Face-LoRA-FFD21E?logo=huggingface&logoColor=black)](https://huggingface.co/yuxinlu1)
[![License: MIT](https://img.shields.io/badge/License-MIT-22c55e)](./LICENSE)

[简体中文](./README.md) · [English](./README_EN.md) · [GitHub](https://github.com/DuckTraDo/Novel) · [LoRA 权重](https://huggingface.co/yuxinlu1)

</div>

---

## ✨ 为什么用它

- 🖥️ **桌面应用，开箱即用** — 装好安装包，填一个模型地址就能写，**无需 Python、无需 clone 仓库**。
- 🧠 **结构化长期记忆** — 人物、伏笔、时间线、事件、章节摘要自动维护，专治长篇「写崩」。
- ✍️ **一句话生成整章** — 给一句 idea，模型起草完整章节；idea 永远是最高优先级。
- 🔍 **连续性审查** — 内置「编辑」帮你揪信息超前、设定漂移、伏笔遗漏、AI 腔。
- 🎨 **好看又顺手** — 中 / 英双语，浅 / 深 / 暖三套主题，2026 现代界面。
- 📂 **文件型，简单稳定** — 全是 YAML / JSON / Markdown，无数据库，可读可备份可手改。
- 🔌 **接本地推理** — OpenAI 兼容接口，llama.cpp / vLLM 等皆可；可配 LoRA 文风。
- 💳 **连 LLM 三选一** — OpenAI 兼容 API，或直接用 **Claude Code / Codex 订阅**额度写小说（调本机已登录的官方 CLI，走 Pro/Max、ChatGPT 订阅，不另花 API Key 的钱）。

> 📣 项目持续维护中,欢迎到 [Issues](https://github.com/DuckTraDo/Novel/issues) 反馈,我会根据反馈**及时修复和更新**。

---

## 🖼️ 界面预览

<div align="center">

<table>
<tr>
<td align="center"><b>章节工作台</b></td>
<td align="center"><b>记忆库</b></td>
</tr>
<tr>
<td><img src="docs/screenshots/workbench.png" width="430" alt="章节工作台" /></td>
<td><img src="docs/screenshots/memory.png" width="430" alt="记忆库" /></td>
</tr>
</table>

</div>

---

## 🖥️ Sodarie Novel 桌面应用

`app/` 是基于 **Tauri（Rust + React）** 的桌面应用。引擎已用 **Rust 原生重写**，
终端用户只需要两样东西：

1. 安装包 `Sodarie Novel_<版本>_x64-setup.exe`
2. 一个能写字的模型——三选一：OpenAI 兼容服务地址（自备本地推理，如 llama.cpp / vLLM），或本机已登录订阅的 **Claude Code / Codex CLI**（见下方「三种 LLM 连接方式」）

首次启动会自动在用户数据目录建好项目并写入模板。

### 🚀 装好即用（终端用户）

从 [**Releases**](https://github.com/DuckTraDo/Novel/releases) 下载对应平台安装包：Windows `*-setup.exe` · macOS `*.dmg`（arm64 / x64）· Linux `*.AppImage` / `*.deb`。

1. 安装并打开 **Sodarie Novel**。
2. 进入 **设置**，选 **LLM 路线**：
   - **API（本地 / OpenAI 兼容）**：填 **LLM Base URL**（须含 `http://`，多数本地服务在 `/v1` 下，例如 `http://127.0.0.1:18180/v1`）与模型名称。
   - **Claude Code / Codex 订阅**：填模型名（留空用订阅默认），无需 Base URL / API Key。
3. 回到 **章节工作台**，新建 `ch001` → 写一句 idea → **生成**。开写！✨

> 第一次用？应用内左侧「📖 使用指南」有详细中英文教程，照着走五分钟上手。

### 🔌 三种 LLM 连接方式

在「设置 → LLM 路线」里三选一：

| 路线 | 怎么连 | 适合谁 |
| --- | --- | --- |
| **API（本地 / OpenAI 兼容）** | 填模型服务地址 + 模型名（可选 API Key） | 自备本地推理（llama.cpp / vLLM）或按量计费 API |
| **Claude Code 订阅** | 装 [Claude Code](https://www.npmjs.com/package/@anthropic-ai/claude-code) 并用 Pro/Max 账号登录（终端跑 `claude` 登录） | 已有 Claude Pro/Max 订阅 |
| **Codex 订阅（ChatGPT）** | 装 [Codex CLI](https://www.npmjs.com/package/@openai/codex) 并用 ChatGPT 登录（终端跑 `codex login`） | 已有 ChatGPT Plus/Pro 订阅 |

> 订阅路线把本机**已登录订阅的官方 CLI**（`claude` / `codex`）当子进程调用，走 **Pro/Max、ChatGPT 订阅额度，不另花 API Key 的钱**；调用时会忽略 API Key 字段，强制走订阅凭据。前提是客户机已装好对应 CLI 并登录过订阅——没装/没登录会返回清晰的中文报错与指引。采样参数（temperature / top_p 等）在订阅路线由 CLI 自管，不可调。

### 🧭 界面功能

| 区域 | 作用 |
| --- | --- |
| ✍️ 章节工作台 | 选章 / 新建 → 一句 idea + 目标字数 → 生成 / 重新生成 → 正文可编辑保存 → 一致性检查 / 更新记忆 / 重置 |
| 📋 报告 | 界面内渲染生成报告、一致性报告、记忆更新报告 |
| 🧠 记忆库 | 编辑 story bible、角色、大纲、伏笔、风格库，降低手写 YAML 门槛 |
| ⚙️ 设置 | 项目目录、LLM 路线（API / Claude Code 订阅 / Codex 订阅）、地址 / 密钥 / 模型；中英文 & 主题切换 |
| 📖 使用指南 | 内置详细中英文操作教程 |

### 🛠️ 从源码开发 / 打包

环境：Node.js 18+ 与 npm、Rust 工具链（`rustc` / `cargo`）。

```powershell
cd app
npm install
npm run tauri dev      # 开发模式
npm run tauri build    # 打包安装器（NSIS）
```

---

## 🔁 创作流程

```mermaid
flowchart LR
    A["准备记忆库<br/>世界观·人物·大纲"] --> B["写一句<br/>chapter idea"]
    B --> C["生成整章"]
    C --> D["审阅 / 改 /<br/>重新生成"]
    D --> E["一致性检查"]
    E --> F["更新长期记忆"]
    F --> B
```

> 核心心法：**idea 决定这章写什么，记忆库决定这章和前文对不对得上。**

---

## 🧩 记忆库都有什么

| 文件 | 作用 |
| --- | --- |
| `memory/story_bible.yaml` | 故事圣经：世界观、主题、写作规则、禁止模式 |
| `memory/characters.yaml` | 角色档案：状态、knows、secrets、关系、限制 |
| `outlines/book_outline.yaml` | 全书方向、结构与章节规划 |
| `memory/foreshadowing.yaml` | 伏笔追踪：active / resolved 双态 |
| `memory/style_bank.jsonl` | 文风样例（每行一条 `{id, text}`），模型据此模仿笔调 |
| `memory/events.jsonl` · `timeline.jsonl` · `chapter_summaries.jsonl` | 自动维护的事件 / 时间线 / 摘要账本 |
| `memory/relationships.json` | 人物关系图（节点 + 边） |
| `chapters/<id>/chapter.md` · `outputs/reports/` | 每章正文 · 各类报告 |

> `events / timeline / summaries / relationships` 由「更新记忆」自动写入，一般不用手改。

---

## 🖋️ LoRA 文风权重

已发布多套中文小说文风 LoRA（基于 Qwen3），按题材分别微调，可按需搭配使用：

| 题材 | 模型 |
| --- | --- |
| 现实主义 | [chinese-realistic-fiction-lora-v1](https://huggingface.co/yuxinlu1/qwen3-6-27b-chinese-realistic-fiction-lora-v1) |
| 犯罪悬疑 | [chinese-crime-fiction-lora-v2](https://huggingface.co/yuxinlu1/qwen3-6-27b-chinese-crime-fiction-lora-v2) |
| 民俗恐怖 | [chinese-folk-horror-lora-v2](https://huggingface.co/yuxinlu1/qwen3-6-27b-chinese-folk-horror-lora-v2) |
| 仙侠 | [chinese-xianxia-lora-v2](https://huggingface.co/yuxinlu1/qwen3-6-27b-chinese-xianxia-lora-v2) |

全部模型主页：**<https://huggingface.co/yuxinlu1>**

> LoRA 权重不放进 GitHub 仓库；仓库只放代码、文档和配置模板。

---

## ⌨️ 命令行版（进阶 / 习惯终端的用户）

桌面应用之外，`scripts/*.py` 提供等价的命令行流程（需 Python 3.10+ 与 `openai`、`pyyaml`）。

<details>
<summary>展开命令行用法</summary>

```powershell
# 1. 生成一章
python scripts/generate_chapter_local.py --chapter ch001 --idea "第一章：主角回到故乡，发现父亲留下的一封信，决定调查多年前的旧事。" --target-words 4000 --overwrite

# 2. 一致性检查
python scripts/check_consistency.py --chapter ch001

# 3. 更新长期记忆
python scripts/update_memory_after_chapter.py --chapter ch001

# 4. 重置某章（默认只删正文/报告；加 --include-memory 连带过滤摘要/事件/时间线）
python scripts/reset_chapter.py --chapter ch001
```

`generate_chapter_local.py` 常用参数：

- `--idea` / `--idea-file`：二选一
- `--target-words`：目标中文字数，默认 4000
- `--overwrite`：允许覆盖已有 `chapter.md`
- `--no-context`：不读取长期记忆，只按 idea 生成
- `--dry-run`：只构建 prompt 和报告，不调用 LLM

</details>

---

## 🛡️ 安全与隐私

面向本地写作与本地推理。公开仓库时请注意（`.gitignore` 已默认排除大部分）：

- ❌ 不提交 `.env`、接口密钥、访问令牌等凭证
- ❌ 不提交本地模型 / LoRA 适配器文件
- ❌ 不提交私稿 `chapters/`、生成产物 `outputs/`、本地设置 `.ui-settings.json`
- ❌ 不提交训练数据原文或未授权文本

详见 [`SECURITY_CHECKLIST.md`](./SECURITY_CHECKLIST.md)。

---

## 🗺️ Roadmap

- [x] 文件型记忆系统
- [x] 整章一次生成
- [x] 章节后记忆更新
- [x] 完整章节一致性检查
- [x] 第一版中文小说 LoRA 发布到 Hugging Face
- [x] 桌面应用（Tauri，Rust 引擎，中英双语 + 主题切换）
- [ ] 检索增强记忆（RAG）
- [ ] 多 LoRA 文风库
- [ ] 一键导出书稿

---

## 🤝 反馈 · 贡献 · 许可

> 📣 **持续维护中。** 用着有问题、有想法,欢迎在 [Issues](https://github.com/DuckTraDo/Novel/issues) 提反馈——我会根据大家的反馈**及时修复和更新**。

也欢迎直接发 PR,尤其是 prompt 优化、流水线脚本、一致性检查、本地模型兼容性等方向。

LoRA 文风权重:<https://huggingface.co/yuxinlu1>

License:[MIT](./LICENSE) © 2026 DuckTraDo。

<div align="center">

Made by [**DuckTraDo**](https://github.com/DuckTraDo) · 用 ☕ 与本地大模型写就

</div>
