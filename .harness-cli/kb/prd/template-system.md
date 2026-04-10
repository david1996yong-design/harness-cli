# 嵌入式模板系统

> 在编译时将所有模板文件嵌入二进制文件，使 CLI 工具完全自包含

## 模块概述

模板系统使用 `rust-embed` crate 将 `embedded/templates/` 目录下的所有模板文件在编译时嵌入到二进制文件中。运行时通过类型安全的 API 访问模板内容，无需外部文件依赖。模板分为三类：平台模板（每个 AI 平台一个 Embed struct）、harness-cli 工作流模板、和 markdown 文档模板。Antigravity 没有独立 Embed struct，它复用 Codex 的模板内容并适配为 workflow 术语。

## 关键文件

| 文件路径 | 职责 |
|----------|------|
| `src/templates/mod.rs` | 模板模块导出 |
| `src/templates/extract.rs` | 定义 14 个 `rust_embed::Embed` 结构体（12 个平台 + harness-cli + markdown），提供 `copy_embedded_dir`、`get_embedded_file`、`list_files`、`CopyOptions` |
| `src/templates/markdown.rs` | Markdown 文档模板的类型安全访问器，使用 `md_template!` 宏定义 backend、frontend、guides、kb/prd、kb/tech 的模板函数 |
| `src/templates/harness_cli.rs` | harness-cli 工作流模板访问器：`workflow.md`、`.gitignore`、`config.yaml`、`worktree.yaml`、`scripts/` 列表 |
| `src/templates/claude.rs` | Claude Code 模板访问器（commands、agents、hooks、settings） |
| `src/templates/cursor.rs` | Cursor 模板访问器 |
| `src/templates/codex.rs` | Codex 模板访问器：skills、codex_skills、agents、hooks、config |
| `src/templates/copilot.rs` | Copilot 模板访问器：prompts、hooks、hooks config |
| `src/templates/iflow.rs` | iFlow 模板访问器 |
| `src/templates/opencode.rs` | OpenCode 模板访问器 |
| `src/templates/kilo.rs` | Kilo 模板访问器 |
| `src/templates/kiro.rs` | Kiro 模板访问器 |
| `src/templates/gemini.rs` | Gemini 模板访问器 |
| `src/templates/antigravity.rs` | Antigravity 模板访问器：读取 Codex skills 并通过字符串替换适配为 `.agent/workflows/<name>.md` |
| `src/templates/windsurf.rs` | Windsurf 模板访问器 |
| `src/templates/qoder.rs` | Qoder 模板访问器 |
| `src/templates/codebuddy.rs` | CodeBuddy 模板访问器 |

## 核心功能

### 编译时嵌入

- **业务规则**: 每个平台有独立的 `Embed` 结构体，统一排除 `.ts`、`.js`、`.d.ts`、`.map`、`__pycache__/` 等构建产物；部分平台额外排除 `node_modules/`、`bun.lock`、`.gitignore`
- **触发条件**: 编译时由 `rust-embed` 处理
- **处理流程**: `rust-embed` 扫描 `embedded/templates/<platform>/` 目录，将文件内容嵌入二进制

### 模板解压到磁盘

- **业务规则**: `copy_embedded_dir<T>(dest, &CopyOptions)` 将嵌入资源解压到目标目录
- **触发条件**: 配置器调用 `configure` 时
- **处理流程**:
  1. 确保目标目录存在
  2. 遍历 `T::iter()` 列出的文件
  3. 如 `resolve_placeholders == true` 且匹配 `placeholder_filename`（或未设置文件名），执行占位符替换
  4. 创建父目录并通过 `write_file` 写入
  5. `executable == true` 且文件以 `.sh`/`.py` 结尾时设置可执行权限

### 单文件访问

- **业务规则**: `get_embedded_file<T>(path)` 返回 `Option<String>`，直接读取单个嵌入文件
- **触发条件**: 需要访问单个已知路径的模板时（如 `AGENTS.md`、`config.yaml`）
- **处理流程**: 调用 `T::get(path)`，将字节内容转为 UTF-8 字符串

### 模板文件列表

- **业务规则**: `list_files<T>()` 返回 `Vec<String>`，列出某个 Embed struct 中的所有文件路径
- **触发条件**: 需要枚举嵌入目录时（如收集 `scripts/` 下所有脚本）

### Markdown 文档模板访问

- **业务规则**: 使用 `md_template!` 宏生成静态访问器函数，通过 `OnceLock` 缓存内容
- **触发条件**: scan 命令或 init 命令需要写入文档模板时
- **处理流程**: 首次访问时从 `MarkdownTemplates` 读取，缓存到 `OnceLock` 静态变量

### Antigravity 内容适配

- **业务规则**: Antigravity 没有独立嵌入资源，运行时从 Codex skills 读取内容并替换术语
- **处理流程**: `adapt_skill_content_to_workflow` 将 "Codex skills" -> "Antigravity workflows"、`.agents/skills/` -> `.agent/workflows/` 等

## 数据流

```
编译时：
  embedded/templates/<platform>/ -> rust-embed -> 二进制嵌入
  （antigravity 无独立嵌入）

运行时：
  get_embedded_file<T>(path) -> Option<String>（单文件）
  copy_embedded_dir<T>(dest, &opts) -> 批量解压到磁盘
  md_template! 宏 -> OnceLock 缓存 -> &'static str
  Antigravity: codex::get_all_skills() -> adapt_skill_content_to_workflow
```

## 业务规则

- 模板文件使用 `.md.txt` 后缀避免被编辑器的 markdown 处理器干扰
- 每个平台的 `Embed` 结构体独立定义，允许精细控制排除规则
- `CopyOptions` 支持三个选项：`executable`（设置可执行权限）、`resolve_placeholders`（占位符替换）、`placeholder_filename`（限定替换范围到指定文件名）
- `md_template!` 宏确保每个模板只从嵌入资源读取一次并缓存
- `harness_cli::get_all_scripts()` 只返回 `scripts/` 前缀下的文件
- Antigravity 不需要单独的嵌入资源，因为它复用 Codex 的 skill 内容

## 与其他模块的关系

| 模块 | 关系 |
|------|------|
| platform-configurators | 配置器调用 `copy_embedded_dir` 和按平台的模板访问器 |
| cli-commands | scan 命令使用 markdown 模板，init 命令使用 harness_cli 模板和 markdown 模板 |
| file-management | `copy_embedded_dir` 内部调用 `write_file` |
| ai-tool-registry | `TemplateDir` 枚举映射到嵌入目录名（小写） |
