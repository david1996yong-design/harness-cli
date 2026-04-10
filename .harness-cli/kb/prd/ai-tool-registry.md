# AI 工具注册表

> 定义所有支持的 AI 编程工具及其配置信息的单一数据源

## 模块概述

AI 工具注册表是 harness-cli 的核心数据模型，使用 `LazyLock<HashMap>` 存储 13 个 AI 编程平台的配置信息。所有平台相关的操作（初始化、更新、路径管理、CLI flag 解析）都从这个注册表获取配置数据。注册表同时定义了 `AITool`、`TemplateDir`、`CliFlag` 三个类型安全的枚举。

## 关键文件

| 文件路径 | 职责 |
|----------|------|
| `src/types/ai_tools.rs` | 定义 `AITool`、`TemplateDir`、`CliFlag` 枚举，`AIToolConfig` 结构体，`AI_TOOLS` 静态注册表和公共访问辅助函数 |
| `src/types/mod.rs` | 类型模块导出 |

## 核心功能

### 平台枚举定义

- **业务规则**: `AITool` 枚举包含 13 个变体：ClaudeCode、Cursor、OpenCode、IFlow、Codex、Kilo、Kiro、Gemini、Antigravity、Windsurf、Qoder、CodeBuddy、Copilot
- **触发条件**: 编译时固定
- **处理流程**: 提供 `all()` 方法返回所有变体切片，`as_str()` 返回 kebab-case 标识符，`Display` trait 透传 `as_str()`

### AIToolConfig 字段

- `name`: 人类可读的平台名称（如 "Claude Code"、"GitHub Copilot"）
- `template_dirs`: `TemplateDir` 枚举列表（总是包含 `Common` 和对应平台的目录）
- `config_dir`: 项目根目录下的配置目录（如 `.claude`、`.kiro/skills`）
- `supports_agent_skills`: 可选 bool，标记平台是否额外管理 `.agents/skills/`（目前仅 Codex 为 `Some(true)`）
- `extra_managed_paths`: 可选 String 列表，用于平台管理的额外目录（目前仅 Copilot 使用）
- `cli_flag`: `CliFlag` 枚举值，对应 `--claude`、`--cursor` 等 CLI 标志
- `default_checked`: 交互式 init 菜单中是否默认选中
- `has_python_hooks`: 平台是否使用 Python hooks（影响 Windows 编码检测提示）

### 平台配置查询

- **业务规则**: 通过 `get_tool_config(AITool)` 获取平台的完整配置；所有 `AITool` 变体必须在注册表中存在（启动时 panic 式断言）
- **触发条件**: init/update 命令和所有配置器需要平台信息时
- **处理流程**: 从 `AI_TOOLS` LazyLock HashMap 中查找

### 路径管理

- **业务规则**: `get_managed_paths(AITool)` 返回平台管理的所有目录路径：`config_dir` + 可选 `.agents/skills` + `extra_managed_paths`
- **触发条件**: 判断文件是否属于受管理的路径时
- **处理流程**: 聚合 `config_dir`、当 `supports_agent_skills == Some(true)` 时追加 `.agents/skills`，最后追加 `extra_managed_paths` 中的所有项

### 模板目录查询

- **业务规则**: `get_template_dirs(AITool)` 返回平台需要复制的模板目录列表
- **触发条件**: 配置器复制模板时
- **处理流程**: 从 `AIToolConfig.template_dirs` 字段直接返回切片

## 数据流

```
AITool 枚举变体
  -> AI_TOOLS HashMap 查找
  -> AIToolConfig 结构体
  -> 各业务模块读取配置字段
       - name / default_checked -> 交互式菜单
       - config_dir / extra_managed_paths -> 文件管理
       - template_dirs -> 模板复制
       - cli_flag -> CLI 参数解析
       - has_python_hooks -> Windows 编码提示
```

## 业务规则

- 每个平台必须有唯一的 `config_dir`（通过 `test_unique_config_dirs` 保证）
- 每个平台必须有唯一的 `cli_flag`（通过 `test_unique_cli_flags` 保证）
- 所有平台的 `config_dir` 以 `.` 开头（隐藏目录，通过 `test_config_dirs_start_with_dot` 保证）
- 所有平台的 `template_dirs` 必须包含 `Common`（通过 `test_template_dirs_include_common` 保证）
- 没有平台的 `config_dir` 可以是 `.harness-cli`（避免与工作流目录冲突）
- `supports_agent_skills == Some(true)` 的平台不能把 `config_dir` 设为 `.agents/skills`（避免冲突）
- 目前仅 Codex 启用 `supports_agent_skills`
- 目前仅 Copilot 使用 `extra_managed_paths`：`.github/hooks` 和 `.github/prompts`
- 默认选中（`default_checked = true`）的平台：ClaudeCode 和 Cursor
- 有 Python hooks（`has_python_hooks = true`）的平台：ClaudeCode、IFlow、Codex、Copilot
- 平台特殊 `config_dir`：
  - `Kilo` -> `.kilocode`（不是 `.kilo`）
  - `Kiro` -> `.kiro/skills`
  - `Antigravity` -> `.agent/workflows`
  - `Windsurf` -> `.windsurf/workflows`
  - `Copilot` -> `.github/copilot`
  - 其他平台均为 `.<kebab-name>`（如 `.claude`、`.cursor`、`.opencode`、`.iflow`、`.codex`、`.gemini`、`.qoder`、`.codebuddy`）

## 与其他模块的关系

| 模块 | 关系 |
|------|------|
| cli-commands | init 命令读取注册表生成交互式选择列表和 CLI flag |
| platform-configurators | 根据注册表中的 `template_dirs`/`config_dir` 执行模板复制；通过 `get_managed_paths` 判断受管路径 |
| file-management | 使用 `get_managed_paths` 判断文件是否受管理 |
| template-system | 通过 `TemplateDir` 枚举索引嵌入式模板目录 |
