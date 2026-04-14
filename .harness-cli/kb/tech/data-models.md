# 核心数据结构

## AITool 枚举

- **文件位置**: `src/types/ai_tools.rs:14-29`
- **用途**: 表示支持的 AI CLI 工具，全局唯一枚举

### Schema

```rust
pub enum AITool {
    ClaudeCode, Cursor, OpenCode, IFlow, Codex,
    Kilo, Kiro, Gemini, Antigravity, Windsurf,
    Qoder, CodeBuddy, Copilot,
}
```

13 个变体，通过 `AITool::all()` 返回静态切片。每个变体关联一个 `AIToolConfig`（见下）。

---

## AIToolConfig

- **文件位置**: `src/types/ai_tools.rs:182-199`
- **用途**: 每个 AI 平台的元配置，存储在全局 `LazyLock` 注册表中 (src/types/ai_tools.rs:215)

### Schema

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| name | `&'static str` | 是 | 平台显示名（如 "Claude Code"） |
| template_dirs | `Vec<TemplateDir>` | 是 | 该平台包含的模板目录（Common + Platform-specific） |
| config_dir | `&'static str` | 是 | 配置目录相对路径（如 `.claude`） |
| supports_agent_skills | `Option<bool>` | 否 | 是否支持 `.agents/skills/` 共享层 |
| extra_managed_paths | `Option<Vec<String>>` | 否 | 额外需要由 init 管理的路径 |
| cli_flag | `CliFlag` | 是 | `--claude`、`--cursor` 等 CLI 标志 |
| default_checked | `bool` | 是 | 交互式 MultiSelect 中是否默认勾选 |
| has_python_hooks | `bool` | 是 | 是否需要 Python 环境（影响 Windows 编码） |

---

## CommandTemplate / AgentTemplate / HookTemplate / SettingsTemplate

- **文件位置**: `src/templates/claude.rs:8-34`（其他平台同构）
- **用途**: 动态扫描 rust-embed 资源后的强类型封装，传给 configurator

### Schema

```rust
pub struct CommandTemplate {
    pub name: String,       // 命令名（无 .md 扩展），如 "archive"
    pub content: String,    // Markdown 原文
}

pub struct AgentTemplate {
    pub name: String,       // agent 名（无 .md 扩展），如 "dispatch"
    pub content: String,
}

pub struct HookTemplate {
    pub target_path: String,  // 相对于 .claude/ 的路径，如 "hooks/session-start.py"
    pub content: String,
}

pub struct SettingsTemplate {
    pub target_path: String,  // 如 "settings.json"
    pub content: String,      // 含 {{PYTHON_CMD}} 等 placeholder
}
```

通过 `get_all_commands() / get_all_agents() / get_all_hooks() / get_settings_template()` 获取，后者在 configurator 中被批量写入目标目录。

---

## TaskData (Python TypedDict)

- **文件位置**: `.harness-cli/scripts/common/types.py:21-53`
- **用途**: task.json 的 schema（权威定义）
- **注意事项**: `TaskData` 使用 `total=False`，因此运行时 task.json 可以缺少 TypedDict 中声明的任何字段而不报错。历史上有过 TypedDict 与 `cmd_create` 实际写入字段漂移的情况（例如 `merge_mode` 等 `meta` 字段只在运行时出现）——请以 `cmd_create`（task_store.py）写入的默认对象作为"最完整的实际 schema"准绳

### Schema

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | str | 是 | 唯一标识，如 `01-21-my-task` |
| name | str | 是 | Task 名称 |
| title | str | 是 | 人类可读标题 |
| description | str | 否 | 需求描述 |
| status | str | 是 | `planning` / `in_progress` / `review` / `completed` / `rejected` |
| dev_type | str | 是 | `backend` / `frontend` / `fullstack` / `bugfix` / `refactor` / `docs` / `test` |
| scope | str \| None | 否 | commit message 的作用域，如 `auth` |
| package | str \| None | 否 | monorepo 中的目标包名 |
| priority | str | 是 | `P0` / `P1` / `P2` / `P3` |
| creator | str | 是 | 创建者（取自 `.developer` 文件） |
| assignee | str | 是 | 被指派人 |
| createdAt | str | 是 | ISO 日期，如 `2026-04-11` |
| completedAt | str \| None | 否 | 完成时间 |
| branch | str \| None | 否 | feature branch 名称，如 `feature/update-kb` |
| base_branch | str \| None | 否 | PR 目标分支（默认 `master`） |
| worktree_path | str \| None | 否 | worktree 绝对路径 |
| current_phase | int | 是 | 当前阶段（0 = 未启动，1+ = 进行中），与 `next_action` 联动 |
| next_action | list[dict] | 是 | 阶段数组，见下文 |
| commit | str \| None | 否 | 最后一次 commit SHA |
| pr_url | str \| None | 否 | 创建的 PR URL |
| kb_status | str | 是 | `needed` / `updated` / `not_required`；archive 阻塞当值为 `needed`（详见 PRD 层 kb-system.md 的 KB Status Gate） |
| subtasks | list[str] | 否 | 子任务 ID 列表 |
| children | list[str] | 否 | 子任务目录名列表 |
| parent | str \| None | 否 | 父任务 ID |
| relatedFiles | list[str] | 否 | 相关文件路径 |
| notes | str | 否 | 任务备注 |
| meta | dict | 否 | 扩展字段（direct-merge 会写 `merge_mode`、`merge_target`） |
| merge_mode | str | 否 | `direct`（direct_merge.py 设置） |
| merge_target | str | 否 | 直接 merge 目标分支 |
| merge_commit | str | 否 | merge 后的 commit SHA |

### 示例

```json
{
  "id": "04-10-update-kb",
  "name": "04-10-update-kb",
  "title": "update-kb: 增量更新产品知识库",
  "status": "in_progress",
  "dev_type": "backend",
  "scope": "claude-commands",
  "priority": "P2",
  "creator": "david1996yong",
  "assignee": "david1996yong",
  "createdAt": "2026-04-10",
  "branch": "feature/update-kb",
  "base_branch": "master",
  "worktree_path": "/home/zy/harness-cli-worktrees/feature/update-kb",
  "current_phase": 0,
  "next_action": [
    {"phase": 1, "action": "implement"},
    {"phase": 2, "action": "check"},
    {"phase": 3, "action": "finish"},
    {"phase": 4, "action": "create-pr"}
  ],
  "commit": null,
  "pr_url": null,
  "kb_status": "needed",
  "subtasks": [],
  "children": [],
  "parent": null,
  "relatedFiles": [],
  "notes": "",
  "meta": {}
}
```

---

## next_action 阶段数组

- **文件位置**: task.json 内字段；`common/phase.py:get_phase_for_action()` 读取
- **用途**: 任务状态机的外化表达，AI agent 按序执行

### Schema

```python
next_action: list[{
    "phase": int,      # 1-indexed，顺序执行
    "action": str,     # 预定义 action 名：
                       #   "implement" → implement sub-agent
                       #   "check" → check sub-agent
                       #   "finish" → 规范化 commit message
                       #   "create-pr" → multi_agent/create_pr.py
                       #   "direct-merge" → multi_agent/direct_merge.py
}]
```

### 状态转移

`current_phase` 与 `status` 的联动：

| current_phase | status | 含义 |
|--------------|--------|------|
| 0 | planning | 刚创建，等待 plan agent 填充 PRD |
| 0 | in_progress | start.py 启动后立即设置 |
| 1 | in_progress | implement 阶段进行中 |
| 2 | in_progress | check 阶段进行中 |
| 3 | in_progress | finish 阶段进行中 |
| 4 | completed | direct-merge 或 create-pr 完成 |

---

## TaskInfo (冻结 dataclass)

- **文件位置**: `.harness-cli/scripts/common/types.py:60-96`
- **用途**: 读路径的不可变视图；写路径需要操作 `raw` 字典

### Schema

```python
@dataclass(frozen=True)
class TaskInfo:
    dir_name: str              # 目录名（如 04-10-my-task）
    directory: Path            # 绝对路径
    title: str
    status: str
    assignee: str
    priority: str
    children: tuple[str, ...]  # tuple 确保不可变
    parent: str | None
    package: str | None
    raw: dict                  # 原始 task.json dict
    
    @property
    def name(self) -> str:
        return self.raw.get("name") or self.raw.get("id") or self.dir_name
```

---

## AgentRecord (registry.json 中的条目)

- **文件位置**: `.harness-cli/scripts/common/registry.py`
- **持久化**: `.harness-cli/workspace/{developer}/.agents/registry.json`
- **用途**: 追踪当前运行中的 agent 进程

### Schema

| 字段 | 类型 | 说明 |
|------|------|------|
| id | str | agent ID（通常等于 task ID，`branch.replace("/", "-")`） |
| worktree_path | str | worktree 绝对路径 |
| pid | int | 后台 agent 进程 ID |
| started_at | str | ISO 时间戳，如 `2026-04-11T06:28:52.145969` |
| task_dir | str | 任务目录相对路径，如 `.harness-cli/tasks/04-10-xxx` |
| platform | str | `claude` / `cursor` / `iflow` / `opencode` |

### 示例

```json
{
  "agents": [
    {
      "id": "rescan-prd-kb",
      "worktree_path": "/home/zy/harness-cli-worktrees/feature/rescan-prd-kb",
      "pid": 43982,
      "started_at": "2026-04-11T06:28:52.145969",
      "task_dir": ".harness-cli/tasks/04-10-rescan-prd-kb",
      "platform": "claude"
    }
  ]
}
```

---

## worktree.yaml 配置

- **文件位置**: `.harness-cli/worktree.yaml`
- **解析器**: `.harness-cli/scripts/common/worktree.py`（简单 YAML 解析，无外部依赖）
- **用途**: 并行 worktree 的全局配置

### Schema

```yaml
worktree_dir: ../harness-cli-worktrees  # worktree 存储目录（相对项目根）

copy:                                   # init 时复制到 worktree 的文件列表
  - .harness-cli/.developer
  # - .env
  # - .env.local

post_create:                            # worktree 创建后执行的 shell 命令
  # - npm install
  # - pnpm install --frozen-lockfile

verify:                                 # Ralph Loop 使用的验证命令
  # - pnpm lint
  # - pnpm typecheck
```

---

## config.yaml 配置

- **文件位置**: `.harness-cli/config.yaml`
- **加载函数**: `.harness-cli/scripts/common/config.py::_load_config()`
- **用途**: 项目级配置（session、hooks、monorepo 包定义）

### Schema

```yaml
session_commit_message: "chore: record journal"
max_journal_lines: 2000

hooks:                                  # 任务生命周期 hook
  after_create:
    - echo "Task created"
  after_archive:
    - git tag {{ task_id }}-archived

packages:                               # monorepo 包定义（可选）
  name: "my-monorepo"
  root: "."
  workspaces:
    - path: "packages/frontend"
      type: "frontend"
    - path: "packages/backend"
      type: "backend"
```

加载失败时静默返回空 dict（不阻断执行）。

---

## settings.json (Claude Code)

- **文件位置**: `embedded/templates/claude/settings.json`（模板），部署到 `.claude/settings.json`
- **用途**: Claude Code 的 hook 注册表

### Schema（关键字段）

```json
{
  "statusLine": {
    "type": "command",
    "command": "{{PYTHON_CMD}} .claude/hooks/statusline.py"
  },
  "hooks": {
    "SessionStart": [
      {
        "matcher": "startup|clear|compact",
        "hooks": [
          {
            "type": "command",
            "command": "{{PYTHON_CMD}} .claude/hooks/session-start.py",
            "timeout": 10
          }
        ]
      }
    ],
    "PreToolUse": [
      {
        "matcher": "Task|Agent",
        "hooks": [
          {
            "type": "command",
            "command": "{{PYTHON_CMD}} .claude/hooks/inject-subagent-context.py",
            "timeout": 30
          }
        ]
      }
    ],
    "SubagentStop": [
      {
        "matcher": "check",
        "hooks": [
          {
            "type": "command",
            "command": "{{PYTHON_CMD}} .claude/hooks/ralph-loop.py",
            "timeout": 10
          }
        ]
      }
    ]
  }
}
```

`{{PYTHON_CMD}}` 由 `src/configurators/shared.rs:14-19` 在 init 时替换为 `python3`（Unix）或 `python`（Windows）。

---

## DetectedPackage (monorepo 检测)

- **文件位置**: `src/utils/project_detector.rs:18-29`
- **用途**: init 时检测项目类型和包结构

### Schema

```rust
pub struct DetectedPackage {
    pub name: String,
    pub path: String,       // 相对路径，无 ./ 前缀
    pub type_: ProjectType,
    pub is_submodule: bool,
}

pub enum ProjectType {
    Frontend, Backend, Fullstack, Unknown,
}
```

由 `detect_project_type()` 和 `detect_packages()` 通过扫描 `package.json` / `Cargo.toml` / workspaces 字段产出。

---

## SpecTemplate (远程 marketplace)

- **文件位置**: `src/utils/template_fetcher.rs:34-43`
- **用途**: 从 `https://raw.githubusercontent.com/mindfold-ai/harness-cli/main/marketplace/index.json` 拉取的模板元数据

### Schema

```rust
pub struct SpecTemplate {
    pub id: String,
    pub type_: String,         // "spec" / "skill" / "command" / "full"
    pub name: String,
    pub description: Option<String>,
    pub path: String,          // giget 格式，如 "gh:user/repo/path"
    pub tags: Option<Vec<String>>,
}
```

仅在 `harness-cli init --template <id>` 时使用；offline 时静默降级为 blank templates。
