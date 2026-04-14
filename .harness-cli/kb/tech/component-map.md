# 组件关系

## 依赖关系图

```
┌──────────────────────────────────────────────────────────────────┐
│                     harness-cli (Rust binary)                     │
│                                                                    │
│   src/main.rs  ──┬── commands::init    ──┬── configurators::*     │
│                  │                       │   └── templates::{plat}│
│                  │                       │       └── rust-embed   │
│                  │                       │           └── embedded/│
│                  ├── commands::scan      │                         │
│                  ├── commands::doctor    │  (环境诊断只读)          │
│                  ├── commands::status    │  (项目快照只读)          │
│                  └── commands::update  ──┴── migrations::*         │
│                                              └── embedded/manifests/│
│                                                                    │
│   utils::{project_detector, template_fetcher, template_hash,      │
│           file_writer, compare_versions}                           │
│                                                                    │
│   types::{ai_tools, migration}                                     │
└──────────────────────────────────────────────────────────────────┘
                              │
                              │ 初始化时部署
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│                     .harness-cli/ (运行时)                        │
│                                                                    │
│   scripts/                                                         │
│     ├── task.py ──────────────┐                                    │
│     │  (cmd_finish 触发                                            │
│     │   add_session + refresh                                      │
│     │   global workspace index)                                    │
│     │                         │                                    │
│     ├── add_session.py ───────┤ (journal + 个人 index)             │
│     ├── update_workspace_index.py ┤ (全局 Active Developers 表)    │
│     │                         │                                    │
│     ├── multi_agent/          │                                    │
│     │   ├── plan.py      ─────┤    共用 common/*                  │
│     │   ├── start.py     ─────┤                                    │
│     │   ├── direct_merge ─────┤                                    │
│     │   ├── create_pr    ─────┤                                    │
│     │   └── cleanup.py   ─────┤                                    │
│     │                         │                                    │
│     ├── hooks/                │                                    │
│     │   ├── session-start.py ─┤                                    │
│     │   ├── inject-subagent  ─┤                                    │
│     │   └── ralph-loop.py    ─┤                                    │
│     │                         ▼                                    │
│     └── common/  (paths, git, config, log, registry, io,           │
│                   cli_adapter, task_store, phase, worktree,        │
│                   types, developer, task_utils, ...)               │
│                                                                    │
│   tasks/{task-id}/task.json, prd.md, *.jsonl                       │
│   workspace/{developer}/.agents/registry.json                      │
│   kb/{prd,tech}/                                                   │
└──────────────────────────────────────────────────────────────────┘
                              │
                              │ Agent 运行时调用
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│            AI CLI (Claude Code / Cursor / iFlow / ...)            │
│                                                                    │
│   通过 slash commands (`/hc:*`) 触发 workflow                     │
│   通过 hooks (SessionStart/PreToolUse/SubagentStop) 执行 Python   │
│   通过 Agent tool 派发子 agent（dispatch, plan, implement, ...）   │
└──────────────────────────────────────────────────────────────────┘
```

## 调用链

### Rust binary 侧

| 调用方 | 被调用方 | 方式 | 说明 |
|--------|---------|------|------|
| `src/main.rs` | `commands::init::init()` | 函数调用 | CLI 子命令 `init` 路由 (src/main.rs:230-253) |
| `commands::init` | `configurators::configure_platform()` | 函数调用 | 逐个平台部署模板 |
| `configurators::mod::configure_platform` | `{platform}::configure(cwd)` | match 分发 | 13 分支 (src/configurators/mod.rs:100-116) |
| `configurators::claude::configure` | `copy_embedded_dir::<ClaudeTemplates>()` | 泛型调用 | 把嵌入的模板拷到 `.claude/` (src/configurators/claude.rs:17-27) |
| `copy_embedded_dir` | `rust_embed::Embed::iter() / get()` | trait 方法 | 遍历 `embedded/templates/claude/` 并写文件 |
| `templates::claude::get_all_commands` | `list_files::<ClaudeTemplates>()` | 泛型函数 | 动态扫描 `commands/hc/*.md` (src/templates/claude.rs:37-52) |
| `commands::update` | `migrations::apply_migrations()` | 函数调用 | 读取 `embedded/manifests/*.json` 应用迁移 |
| `utils::project_detector` | `regex::Regex` | 库调用 | monorepo 包检测 |

### Python 运行时侧

| 调用方 | 被调用方 | 方式 | 说明 |
|--------|---------|------|------|
| Dispatch Agent (AI) | `.harness-cli/scripts/task.py <cmd>` | subprocess | 通过 dispatch.md 中的 bash 指令 |
| `task.py` | `common.task_store::cmd_*` | Python import | 任务 CRUD |
| `common.task_store` | `common.io.{read,write}_json` | 函数调用 | task.json 持久化 |
| `common.task_store` | `common.paths.get_repo_root()` | 函数调用 | 找到 `.harness-cli` 所在目录 |
| Dispatch Agent | `multi_agent/plan.py` | subprocess | 进入规划阶段 |
| `plan.py` | `common.cli_adapter.build_run_command()` | 函数调用 | 构造平台特定的 CLI 启动命令 |
| `plan.py` | `subprocess.Popen([claude, -p, ...])` | 子进程 | 后台启动 Plan Agent |
| Dispatch Agent | `multi_agent/start.py` | subprocess | 进入调度阶段 |
| `start.py` | `run_git(["worktree", "add", ...])` | 子进程 | 创建 git worktree |
| `start.py` | `common.registry.registry_add_agent()` | 函数调用 | 注册到 registry.json |
| `start.py` | `subprocess.Popen([claude, ...], cwd=worktree)` | 子进程 | 在 worktree 中启动 Dispatch Agent |
| Dispatch Agent | `multi_agent/direct_merge.py` 或 `create_pr.py` | subprocess | 进入完成阶段 |
| `direct_merge.py` | `run_git(["merge", "--no-ff", ...], cwd=main_repo)` | 子进程 | 在主仓库完成 merge |
| `direct_merge.py` | `common.paths.get_main_repo_root()` | 函数调用 | 解析 worktree 的 `.git` 文件定位主仓 |
| `create_pr.py` | `subprocess.run(["gh", "pr", "create", ...])` | 子进程 | 调用 GitHub CLI |

### Claude Code hooks 触发链

| 触发事件 | 匹配器 | 执行脚本 | 超时 |
|---------|--------|---------|------|
| SessionStart | startup/clear/compact | `hooks/session-start.py` | 10s |
| PreToolUse | Task/Agent | `hooks/inject-subagent-context.py` | 30s |
| SubagentStop | check | `hooks/ralph-loop.py` | 10s |
| statusLine | (持续) | `hooks/statusline.py` | - |

定义位置：`embedded/templates/claude/settings.json:2-73`

## 数据流

### 流 1：项目初始化流

```
用户 CLI 输入
  └─ harness-cli init --claude --yes --force
      └─ parse CLI args (clap)
          └─ detect_project_type() → ProjectType::Rust
              └─ configure_platform(ClaudeCode, cwd)
                  └─ copy_embedded_dir::<ClaudeTemplates>()
                      └─ 写入 .claude/commands/hc/*.md
                      └─ 写入 .claude/agents/*.md
                      └─ 写入 .claude/hooks/*.py
                      └─ 写入 .claude/settings.json（含 {{PYTHON_CMD}} 解析）
              └─ copy_embedded_dir::<HarnessCliTemplates>()
                  └─ 写入 .harness-cli/scripts/
                  └─ 写入 .harness-cli/config.yaml 等
              └─ 创建 bootstrap task（引导新用户）
              └─ 写入 .version 记录当前 CLI 版本
```

### 流 2：任务生命周期流

```
用户 /hc:start "实现登录功能"
  └─ Dispatch Agent 读取 .current-task
      └─ Agent 调用 task.py create → 写 task.json (status=planning)
          └─ Agent 调用 multi_agent/plan.py
              └─ 后台启动 Plan Agent → 产出 prd.md + 完整 task.json
                  └─ task.json 包含 next_action = [
                       {phase:1, action:implement},
                       {phase:2, action:check},
                       {phase:3, action:finish},
                       {phase:4, action:direct-merge/create-pr}
                     ]
      └─ Agent 调用 multi_agent/start.py <task-dir>
          └─ git worktree add <path> <branch>
          └─ shutil.copytree(task_dir, worktree/task_dir)
          └─ registry.json 新增 agent 记录
          └─ subprocess.Popen(claude, cwd=worktree) → 子 Dispatch Agent
              └─ 读取 next_action[current_phase-1] 执行
                  ├─ phase 1: implement sub-agent → 写代码 + jsonl 日志
                  ├─ phase 2: check sub-agent → 验证
                  ├─ phase 3: finish → 规范化 commit message
                  └─ phase 4: direct_merge.py / create_pr.py
                      └─ stage + commit + push
                      └─ git merge (in main_repo_root)
                      └─ 更新 task.json status=completed
                      └─ 同步更新主仓库的 task.json（修复过的 bug）
      └─ 用户 task.py finish
          └─ _finalize_task_on_finish → task.json 字段回写
          └─ _auto_record_session(task_json, repo_root)
              ├─ add_session_from_task → journal + 个人 index
              └─ refresh_global_workspace_index → 全局 Active Developers 表
          └─ clear_current_task
          └─ after_finish hook

      └─ 用户 /hc:archive → task.py archive <dir_name>
          └─ KB Status Gate (task.json.kb_status 必须 ≠ needed 否则 exit 1)
          └─ subtask 双向链接清理
          └─ 物理移动到 tasks/archive/YYYY-MM/
          └─ auto-commit "chore(task): archive <name>"
          └─ refresh_global_workspace_index → 归档任务从全局表移除
```

### 流 3：KB 状态流转

```
task.py create → task.json.kb_status = "needed"（默认）
      │
      ▼
   (开发过程中)
      │
      │   Path A: 任务涉及业务代码变更
      │     └─ AI 运行 /hc:scan-kb 刷新 kb/prd/
      │        └─ AI 顺手执行 task.py mark-kb updated <task>
      │
      │   Path B: 任务不影响 KB（docs-only / test / 重构）
      │     └─ AI/用户 task.py mark-kb not-required <task>
      ▼
   kb_status ∈ {updated, not_required}
      │
      ▼
   task.py archive → 通过 gate 放行
```

### 流 4：hook 注入流（运行时）

```
Claude Code 启动会话
  └─ SessionStart hook 触发（matcher=startup）
      └─ python3 .claude/hooks/session-start.py
          └─ 读取 .harness-cli/.current-task
          └─ 读取 task.json
          └─ 输出 session 上下文（当前任务 / 开发者 / git 状态）到 stdout
      → Claude 把输出作为额外系统提示词注入

用户触发 Task 工具（派发子 agent）
  └─ PreToolUse hook 触发（matcher=Task）
      └─ python3 .claude/hooks/inject-subagent-context.py
          └─ 根据子 agent 类型读取对应 .jsonl（implement.jsonl / debug.jsonl）
          └─ 注入 prd.md + spec/ 相关文件到子 agent 上下文

子 agent 完成
  └─ SubagentStop hook 触发（matcher=check）
      └─ python3 .claude/hooks/ralph-loop.py
          └─ 验证输出是否符合 .harness-cli/worktree.yaml 的 verify 配置
```

## 依赖方向原则

**Rust 侧**
- `src/main.rs` → `commands::*` → `configurators::*` → `templates::*` → `rust-embed` 资源
- `commands::*` → `utils::*` / `types::*`
- `utils::project_detector` 独立无依赖其他模块
- `migrations` 依赖 `embedded/manifests/` 与 `templates::harness_cli`
- **单向依赖**：`main` → `commands` → `configurators` → `templates`，无反向引用

**Python 侧**
- `multi_agent/*.py` → `common/*`（所有 multi_agent 脚本都只向下依赖 common）
- `hooks/*.py` → `common/*`
- `task.py` → `common/task_store` → `common/io, paths, task_utils, config`
- `common/*` 之间有有限的内部依赖：
  - `task_store` → `task_utils, io, paths, types, config`
  - `start.py` → `cli_adapter, registry, worktree, config, paths, git, log`
- **共享底层**：`common/paths.py`、`common/log.py`、`common/io.py` 被广泛依赖，但自身依赖极少

**Rust ↔ Python 边界**
- Rust 只生产 Python 脚本（在 init 时部署），**不调用** Python
- Python 不感知 Rust binary，只在运行时被 AI agent 或 hook 触发
- 两者通过**文件系统**通信：Rust 写 `.harness-cli/scripts/`，Python 读写 `.harness-cli/tasks/`、`.harness-cli/workspace/`

## 架构分层

```
Layer 4  │ AI Agents (Claude/Cursor runtime)  — 外部执行体
─────────┼──────────────────────────────────
Layer 3  │ Python scripts (.harness-cli/scripts/)
         │   - multi_agent/*  (流程编排)
         │   - hooks/*  (上下文注入)
         │   - task.py  (任务 CRUD)
─────────┼──────────────────────────────────
Layer 2  │ Python common library (.harness-cli/scripts/common/)
         │   - paths, io, git, log, config, task_store, ...
─────────┼──────────────────────────────────
Layer 1  │ Rust binary (harness-cli)
         │   - commands, configurators, templates, migrations
─────────┼──────────────────────────────────
Layer 0  │ Embedded resources (rust-embed → binary)
         │   - embedded/templates/*
         │   - embedded/manifests/*
```

下层对上层一无所知（Layer 1 不知道 Layer 3 怎么用它部署的脚本），上层只通过文件系统与下层交互。
