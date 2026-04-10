# 平台配置器

> 为 13 个 AI 编程平台复制嵌入式模板到用户项目目录

## 模块概述

平台配置器是 init 命令的执行层，负责将编译时嵌入的模板文件复制到用户项目的对应目录中。每个平台有一个独立的配置器模块，通过统一的 `configure(cwd)` 接口调用。模块注册中心 `mod.rs` 还提供平台枚举、路径管理、模板收集、CLI flag 解析和交互式选择项构建等辅助函数。此外 `workflow.rs` 负责 `.harness-cli/` 目录树的创建。

## 关键文件

| 文件路径 | 职责 |
|----------|------|
| `src/configurators/mod.rs` | 配置器注册中心：`configure_platform` 调度、`all_managed_dirs`、`is_managed_path`、`resolve_cli_flag`、`get_init_tool_choices` 等 |
| `src/configurators/shared.rs` | 共享工具：`resolve_placeholders` 处理 `{{PYTHON_CMD}}` 占位符 |
| `src/configurators/workflow.rs` | `create_workflow_structure` 创建 `.harness-cli/` 目录树、spec 模板和 monorepo 包目录 |
| `src/configurators/claude.rs` | Claude Code 配置器：复制模板到 `.claude/` |
| `src/configurators/cursor.rs` | Cursor 配置器：复制模板到 `.cursor/` |
| `src/configurators/codex.rs` | Codex 配置器：写入 `.agents/skills/`、`.codex/skills/`、`.codex/agents/`、`.codex/hooks/`、`.codex/hooks.json`、`.codex/config.toml` |
| `src/configurators/copilot.rs` | GitHub Copilot 配置器：写入 `.github/prompts/*.prompt.md`、`.github/copilot/hooks/`、`.github/copilot/hooks.json`、`.github/hooks/harness-cli.json` |
| `src/configurators/iflow.rs` | iFlow CLI 配置器 |
| `src/configurators/opencode.rs` | OpenCode 配置器（通过插件系统，不追踪模板哈希） |
| `src/configurators/kilo.rs` | Kilo CLI 配置器 |
| `src/configurators/kiro.rs` | Kiro Code 配置器（`.kiro/skills`） |
| `src/configurators/gemini.rs` | Gemini CLI 配置器 |
| `src/configurators/antigravity.rs` | Antigravity 配置器：复用 Codex skills 并适配为 `.agent/workflows/<name>.md` |
| `src/configurators/windsurf.rs` | Windsurf 配置器：`.windsurf/workflows` |
| `src/configurators/qoder.rs` | Qoder 配置器 |
| `src/configurators/codebuddy.rs` | CodeBuddy 配置器 |

## 核心功能

### 平台配置调度

- **业务规则**: `configure_platform(AITool, cwd)` 根据平台枚举 match 到对应的配置器
- **触发条件**: init 命令选择平台后依次调用
- **处理流程**: match AITool -> 调用对应平台的 `configure(cwd)` -> 配置器内部使用 `copy_embedded_dir` 或按文件写入嵌入内容

### 工作流目录创建

- **业务规则**: `create_workflow_structure(cwd, options)` 根据 `WorkflowOptions` 创建 `.harness-cli/` 完整目录树
- **触发条件**: 由外部（当前 init 命令仅使用其构件；完整调用在测试中验证）
- **处理流程**:
  1. 创建 `.harness-cli/` 基础目录
  2. 写入 `workflow.md`、`.gitignore`、`config.yaml`
  3. 复制所有 Python/Shell 脚本到 `scripts/`（带可执行权限）
  4. 创建 `workspace/index.md`
  5. 创建 `tasks/` 空目录
  6. 可选写入 `worktree.yaml`（多代理模式）
  7. 根据项目类型创建 spec 模板：`spec/backend/`、`spec/frontend/`、`spec/guides/`
  8. Monorepo 模式下为每个 non-remote 包创建 `spec/<pkg>/` 子目录

### 受管路径查询

- **业务规则**: `all_managed_dirs()` 返回所有受管理的目录列表（始终以 `.harness-cli` 开头，再追加所有平台的 `get_managed_paths`，去重）
- **触发条件**: `update` 命令初始化哈希时
- **处理流程**: 遍历 `AITool::all()`，对每个工具调用 `get_managed_paths`，使用 HashSet 去重

### 路径归属判断

- **业务规则**: `is_managed_path(dir_path)` 判断给定路径是否属于任意受管目录
- **处理流程**: 将 `\` 规范化为 `/`，然后检查路径是否等于或以任一受管目录 + `/` 开头
- 另外 `is_managed_root_dir(dir_name)` 判断根目录名是否为受管目录

### 平台检测

- **业务规则**: `get_configured_platforms(cwd)` 返回 `cwd` 下所有已存在 `config_dir` 的平台集合
- **触发条件**: 需要判断已配置平台时（如 update 命令）
- **处理流程**: 遍历 `AITool::all()`，检查每个 `config_dir` 目录是否存在

### Python hooks 平台过滤

- **业务规则**: `get_platforms_with_python_hooks()` 返回所有 `has_python_hooks == true` 的平台
- **触发条件**: Windows 平台上提示用户使用 `python` 命令

### 交互式选择项构建

- **业务规则**: `get_init_tool_choices()` 遍历所有 `AITool`，构建 `InitToolChoice` 列表（含 key、name、默认勾选、platform_id）
- **触发条件**: init 命令的 MultiSelect 提示
- **处理流程**: 从注册表读取每个工具的 `AIToolConfig`

### CLI flag 解析

- **业务规则**: `resolve_cli_flag(flag)` 将字符串 flag（如 `"claude"`）解析为 `AITool` 变体（大小写敏感）
- **触发条件**: 解析 CLI 参数时
- **处理流程**: 线性扫描所有 `AITool`，比较 `cli_flag.as_str()`

### 模板收集（更新追踪）

- **业务规则**: `collect_platform_templates(AITool)` 收集平台所有模板文件及内容，用于哈希比较
- **触发条件**: update 命令需要对比模板变更时
- **处理流程**: 返回 `HashMap<路径, 内容>`；`OpenCode` 返回 `None`（使用插件系统不追踪）

### 占位符替换

- **业务规则**: 模板中的 `{{PYTHON_CMD}}` 根据操作系统替换为 `python3`（Unix）或 `python`（Windows）
- **触发条件**: 复制含占位符的模板文件时（如 Codex/Copilot 的 `hooks.json`）
- **处理流程**: `resolve_placeholders(content)` 执行字符串替换

## 数据流

```
init 命令选择平台列表
  -> configure_platform(AITool, cwd)
  -> 对应配置器使用 copy_embedded_dir 或按文件写入
  -> 嵌入式模板解压到项目目录
  -> 可选占位符替换（Codex hooks.json / Copilot hooks.json）

update 命令哈希追踪
  -> all_managed_dirs() -> initialize_hashes/update_hashes
  -> collect_platform_templates() -> HashMap 比对
```

## 业务规则

- spec 模板根据项目类型选择：`Backend` 只写入 backend 文档，`Frontend` 只写入 frontend 文档，`Fullstack`/`Unknown` 两者都写入
- `guides/` 目录总是创建（不受项目类型影响），包含 `index.md`、`cross-layer-thinking-guide.md`、`code-reuse-thinking-guide.md`
- Monorepo 模式下为每个 non-remote 包创建独立的 `spec/<pkg>/` 目录，`Unknown` 类型包视为 Fullstack
- 脚本文件（`.py`/`.sh`）通过 `write_file(..., executable=true)` 设置 0755 权限
- `skip_spec_templates` 模式（使用远程模板）下跳过本地 spec 模板创建
- `OpenCode` 使用插件系统，不参与哈希追踪（`collect_templates` 返回 `None`）
- `Copilot` 同时写入两份 `hooks.json`：`.github/copilot/hooks.json`（追踪）和 `.github/hooks/harness-cli.json`（VS Code 发现）
- `Codex` 分别写入共享 skills 到 `.agents/skills/` 和 Codex 专用 skills 到 `.codex/skills/`
- `Antigravity` 复用 Codex 的 skill 内容，并将术语替换为 workflow（`.agent/workflows/<name>.md`）

## 与其他模块的关系

| 模块 | 关系 |
|------|------|
| cli-commands | 被 init 命令调用 |
| ai-tool-registry | 读取平台配置信息和路径管理 |
| template-system | 使用嵌入式模板资源（`copy_embedded_dir`、按平台的模板访问器） |
| file-management | 所有配置器通过 `write_file` 写入文件 |
| project-detection | `workflow.rs` 根据 `ProjectType` 决定 spec 结构 |
