# Security Checklist — GitHub Upload Pre-flight

上传前请逐项确认。

## 1. 敏感文件

- [ ] 确认 `.env` / `.env.*` 不存在于 repo 中
- [ ] 确认无 `*.key` / `*.pem` / `secrets.*` 文件
- [ ] 确认无 `config.local.yaml` 文件
- [ ] 确认 `config.yaml` 中不包含真实 API key 或密码

## 2. 模型文件

- [ ] 确认无 `*.gguf` / `*.safetensors` / `*.bin` / `*.pt` / `*.pth` / `*.ckpt` / `*.onnx` 文件
- [ ] 确认无 `models/` 目录

## 3. 生成内容（私人小说手稿）

- [ ] 确认 `chapters/` 目录已被 .gitignore 排除
- [ ] 确认 `outputs/contexts/` 和 `outputs/reports/` 已被 .gitignore 排除
- [ ] 确认 `inputs/` 目录已被 .gitignore 排除

## 4. 代码中的敏感信息

- [ ] `README.md` 中的模型名为占位符（非真实模型文件名）
- [ ] 代码中无硬编码的 API key、密码、token
- [ ] 代码中无硬编码的本地路径（如 `C:\Users\...`）

## 5. .gitignore 覆盖

.gitignore 已覆盖以下类别：

| 类别 | 模式 |
|------|------|
| Python 缓存 | `__pycache__/`, `*.pyc` |
| 环境/密钥 | `.env`, `*.key`, `*.pem` |
| 模型文件 | `*.gguf`, `*.safetensors`, `*.bin` 等 |
| LoRA 文件 | `*.lora`, `loras/` |
| 小说正文 | `chapters/` |
| Pipeline 输出 | `outputs/contexts/`, `outputs/reports/` |
| 输入草稿 | `inputs/` |
| OS/编辑器 | `.DS_Store`, `.vscode/`, `.idea/` |
| 日志/临时 | `*.log`, `*.tmp`, `*.bak` |
| 测试产物 | `test_*.py` |

## 6. 上传前最后检查

```powershell
# 确认 git status 不包含敏感文件
git status

# 确认 git diff 不包含密钥
git diff --cached

# 检查大文件（模型文件通常是 MB-GB 级别）
git ls-files --others --ignored --exclude-standard
```

## 7. 如果密钥已泄露

如果误将 API key 或密码提交到了 Git 历史：

1. **立即轮换密钥**（在提供商后台重新生成）
2. 使用 `git filter-branch` 或 `git filter-repo` 清理历史
3. Force push 到 remote
4. 在 GitHub 上清除相关的 secret scanning alerts

## 注意

- LoRA 适配器和模型权重文件体积大且可能包含训练数据，不要上传
- 生成的小说正文是私人创作内容，不要上传
- `inputs/` 目录可能包含未完成的草稿，不要上传
