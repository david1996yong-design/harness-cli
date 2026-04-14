# 任务生命周期

> Python 运行时工作流的核心：通过 create / start / finish / archive 四个动作管理 task.json 的生命周期

## 模块概述

任务生命周期模块是开发者与 harness-cli 日常交互的主路径。每项工作都以 `.harness-cli/tasks/<MM-DD-slug>/` 形式组织，由 `task.json` 承载元数据，由 `.harness-cli/.current-task` 指向"当前活跃"任务。四个 CLI 动作（create / start / finish / archive）完整覆盖任务从创建到归档的流转；每个动作都会维护状态字段、运行生命周期钩子、并触发下游副作用（session 记录、全局索引刷新等）。

## 关键文件

| 文件路径 | 职责 |
|----------|------|
| `.harness-cli/scripts/task.py` | CLI 入口与分发器；定义 `cmd_start` / `cmd_finish` 以及 `_promote_status_on_start` / `_finalize_task_on_finish` / `_auto_record_session` 等生命周期辅助函数 |
| `.harness-cli/scripts/common/task_store.py` | `cmd_create` / `cmd_archive` / `cmd_set_branch` / `cmd_set_scope` / `cmd_mark_kb` / `cmd_add_subtask` / `cmd_remove_subtask` —— 所有改动 task.json 的业务命令 |
| `.harness-cli/scripts/common/task_utils.py` | `archive_task_complete` / `run_task_hooks` / `refresh_global_workspace_index` / `resolve_task_dir` / `find_task_by_name` 工具函数 |
| `.harness-cli/scripts/common/tasks.py` | `load_task` / `iter_active_tasks` / `children_progress`：task.json 的统一读取入口 |
| `.harness-cli/scripts/common/paths.py` | 当前任务指针相关：`get_current_task` / `set_current_task` / `clear_current_task` / `normalize_task_ref` |
| `.harness-cli/scripts/common/types.py` | `TaskData` TypedDict（task.json schema）与 `TaskInfo` 冻结数据类 |
| `.harness-cli/.current-task` | 单文件指针，记录当前活跃任务的仓库相对 POSIX 路径（归一化格式） |

## 核心功能

### 1. 五态生命周期

任务状态（task.json `status` 字段）按顺序流转：

```
planning → in_progress → review → completed → (archived/moved to tasks/archive/YYYY-MM/)
```

状态字段不支持用户手动修改，只能通过对应的生命周期命令推进：

| 触发 | 状态变化 | 实现 |
|------|---------|------|
| `task.py create` | 新建 → `planning` | `cmd_create`（task_store.py） |
| `task.py start` | `planning` → `in_progress`（幂等） | `_promote_status_on_start`（task.py） |
| `task.py finish` | `*` → `completed` + 字段回写 | `_finalize_task_on_finish`（task.py） |
| `task.py archive` | completed → 目录移入 `tasks/archive/YYYY-MM/` | `cmd_archive`（task_store.py） |

### 2. create —— 新建任务

- **业务规则**：按 `MM-DD-<slug>` 生成任务目录，写入标准 task.json 和自动生成的 prd.md 模板
- **触发条件**：`python3 task.py create "<title>" [--slug <name>] [--assignee <dev>] [--priority P0|P1|P2|P3] [--parent <dir>] [--package <pkg>]`
- **处理流程**：
  1. 在 `.harness-cli/tasks/MM-DD-<slug>/` 下写 `task.json`，默认 `status=planning` / `kb_status=needed` / `priority=P2` / assignee 取当前 `.developer`
  2. 依据 `kb/prd/index.md` 的模块表自动生成 `prd.md` 模板（含可填写段落）
  3. 若传入 `--parent`：读写父任务 `children` 列表、写本任务 `parent` 字段（建立双向链接）
  4. 触发 `after_create` 钩子
  5. 打印任务相对路径便于脚本拼接

### 3. start —— 设为当前任务

- **业务规则**：写 `.current-task` 指针，并把 `planning` 幂等晋升为 `in_progress`
- **触发条件**：`python3 task.py start <dir>`（支持任务名、相对路径、绝对路径）
- **处理流程**：
  1. `resolve_task_dir()` 解析任务目录
  2. 归一化为仓库相对 POSIX 路径写入 `.harness-cli/.current-task`
  3. 调用 `_promote_status_on_start`：若 `status == planning` 则改为 `in_progress`（幂等）
  4. 触发 `after_start` 钩子

### 4. finish —— 完成任务

- **业务规则**：批量回写"任务完成"类字段，然后触发自动化副作用（session 记录、全局索引刷新），最后清除 current-task 指针
- **触发条件**：`python3 task.py finish`
- **处理流程**（顺序严格）：
  1. 读 `.current-task` 指针，定位 task.json
  2. 调用 `_finalize_task_on_finish` 做**字段回写**（幂等）：
     - `status` → `completed`
     - `completedAt` → 当天日期（若空）
     - `commit` → `HEAD` hash（若空；有 worktree_path 则取 worktree 的 HEAD，否则取主仓库 HEAD）
     - `current_phase` → `len(next_action)`（终态值）
  3. 调用 `_auto_record_session(task_json_path, repo_root)` —— 详见 [session-recording.md](./session-recording.md)（本模块不复述其流程，只需知道它会把 session 写进 journal、刷新个人 index、再刷新全局 index）
  4. 清除 `.current-task`
  5. 触发 `after_finish` 钩子

### 5. archive —— 归档任务

- **业务规则**：归档前强制校验 KB 状态 → 处理子任务双向链接 → 移动目录 → 自动提交 → 刷新全局索引
- **触发条件**：`python3 task.py archive <task-name> [--no-commit]`
- **处理流程**（顺序严格）：
  1. `find_task_by_name` 定位任务（支持精确匹配/后缀匹配）
  2. **KB Status Gate**：读 task.json 的 `kb_status`；若为 `needed` 立即打印错误、返回 exit 1（详细规则见 [kb-system.md](./kb-system.md) 的「KB Status Gate」段）
  3. 兜底回写完成字段（模拟 finish 行为，防止用户跳过 finish 直接 archive）
  4. **子任务双向清理**（重要 side-effect）：
     - 若本任务是子任务：从 `parent.children` 列表中移除自己
     - 若本任务是父任务：把每个 `children[*]` 的 `parent` 字段置 `None`（子任务被"孤立"但仍保留）
  5. `archive_task_complete` 用 `shutil.move` 将目录迁到 `.harness-cli/tasks/archive/YYYY-MM/`
  6. 除非 `--no-commit`，执行 `git add -A .harness-cli/tasks/` + `git commit -m "chore(task): archive <name>"`
  7. 触发 `after_archive` 钩子（外部 shell 命令，失败非阻塞）
  8. 调用 `refresh_global_workspace_index`（见 [session-recording.md](./session-recording.md)）让全局表中移除该已归档任务

### 6. task.json 字段全景

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` / `name` | str | 任务 slug，通常等于目录后缀 |
| `title` | str | 人类可读标题 |
| `description` | str | 详细描述（可空） |
| `status` | str | 主状态枚举：planning / in_progress / review / completed |
| `dev_type` | str \| null | backend / frontend / fullstack / test / docs |
| `scope` | str \| null | PR 标题 scope 前缀 |
| `package` | str \| null | monorepo 包名 |
| `priority` | str | P0 / P1 / P2 / P3 |
| `creator` / `assignee` | str | 开发者名 |
| `createdAt` / `completedAt` | str \| null | ISO 日期 |
| `branch` / `base_branch` | str \| null | git 分支与基线分支 |
| `worktree_path` | str \| null | 可选 git worktree 绝对路径 |
| `current_phase` | int | `next_action` 数组索引；finish 时推到末尾 |
| `next_action` | list\[dict\] | 工作流阶段，默认四阶段：implement / check / finish / create-pr |
| `commit` | str \| null | HEAD hash，finish 时自动回写 |
| `pr_url` | str \| null | 关联 PR 链接 |
| **`kb_status`** | str | **needed / updated / not_required**；新建默认 needed；archive 阻塞当值为 needed（详见 [kb-system.md](./kb-system.md)） |
| `subtasks` / `children` | list\[str\] | 子任务目录名列表（`children` 为实际使用字段） |
| `parent` | str \| null | 父任务目录名；每个任务至多一个父 |
| `relatedFiles` | list\[str\] | 相关源文件列表 |
| `notes` | str | 自由文本 |
| `meta` | dict | 预留扩展 |

> 注意：`TaskData` TypedDict（common/types.py）使用 `total=False`，因此即使 schema 层未列出某字段运行时也不会报错；上述表格反映 `cmd_create` 实际写入的完整默认形态。

### 7. 生命周期钩子

钩子通过 `.harness-cli/config.yaml` 的 `hooks:` 段配置，形式为 shell 命令列表。每次执行都会把 `TASK_JSON_PATH` 注入环境变量。

| 事件 | 触发时机 |
|------|---------|
| `after_create` | `cmd_create` 写完 task.json 之后 |
| `after_start` | `cmd_start` 写完 current-task 之后 |
| `after_finish` | `cmd_finish` 清除 current-task 之后（运行 session 记录之后） |
| `after_archive` | `cmd_archive` 迁移目录之后 |

钩子失败非阻塞——打印 `[WARN]` 但不中断主流程；捕获 `Exception` 和 `SystemExit`。实现见 `task_utils.py` 的 `run_task_hooks()`。

### 8. 当前任务指针（.current-task）

- 文件：`.harness-cli/.current-task`（单行仓库相对 POSIX 路径）
- 写入：`start` 命令 → `set_current_task`
- 读取：`finish` / `mark-kb`（缺省时的默认目标）
- 清除：`finish` / `archive`
- 归一化规则（`normalize_task_ref`）：去 `./` 前缀、转 `\` 为 `/`、未带 `.harness-cli/` 前缀时自动补全
- 存储形式永远是仓库相对路径（不是绝对路径）

### 9. 子任务层级

- **数据结构**：父任务有 `children: list[str]`，子任务有 `parent: str | None`（一对多，每个子任务至多一个父）
- **建立链接**：`create --parent <dir>` 创建时直接建立；或 `task.py add-subtask <parent> <child>` 事后建立
- **拆链接**：`task.py remove-subtask <parent> <child>`
- **展示**：`task.py list` 会以缩进树呈现父子；`task.py status` 会附 `[N/M done]` 进度
- **归档时的双向清理**：见第 5 节 archive 流程第 4 步

## 数据流

```
           ┌──── create ──────────┐
           │ 写 task.json          │
           │ kb_status=needed      │
           │ 生成 prd.md           │
 user ─────┤ 建立 parent 链接       │
 CLI       │ after_create hook     │
           └──── [planning] ───────┘
                     │
                     │ task.py start <dir>
                     ↓
           ┌──── start ───────────┐
           │ 写 .current-task      │
           │ planning→in_progress  │
           │ after_start hook      │
           └─── [in_progress] ─────┘
                     │
                     │ (开发进行中；可 set-branch/set-scope 等)
                     │
                     │ task.py finish
                     ↓
           ┌──── finish ──────────┐
           │ _finalize_task       │
           │ (status/completedAt/ │
           │  commit/phase 回写)   │
           │ _auto_record_session │ ───→ 详见 session-recording.md
           │ clear_current_task   │
           │ after_finish hook    │
           └──── [completed] ──────┘
                     │
                     │ task.py archive <name>
                     ↓
           ┌──── archive ─────────┐
           │ KB gate (kb_status)   │ ───→ 详见 kb-system.md
           │ subtask 双向清理      │
           │ 目录移入 archive/     │
           │ auto-commit           │
           │ after_archive hook    │
           │ refresh global index  │ ───→ 详见 session-recording.md
           └──── [archived] ───────┘
```

## 业务规则

- 状态晋升只由专用命令完成，没有通用的"set status"——保证状态机一致性
- `finish` 的字段回写是幂等的，重复 `finish` 不会破坏已有值
- `archive` 额外做一次 finish 的兜底回写，允许用户跳过 finish 直接 archive（容错设计）
- archive 必须 `kb_status != needed`（详见 [kb-system.md](./kb-system.md)），无逃生阀
- 子任务层级只有两级语义，但 `children` 链可以嵌套；归档父任务会"孤儿化"子任务（保留 task 本身只是清掉 parent 字段）
- `find_task_by_name` 采用后缀匹配：`"my-task"` 能找到 `04-14-my-task`；若有歧义返回首个匹配
- `auto-commit on archive` 默认开启；用 `--no-commit` 关闭（多次 archive 合并提交场景）
- current-task 是单活跃任务语义：同一仓库只能有一个任务处于"当前"；并发 `start` 最后写入者胜出（无锁）
- Worktree 场景下 `commit` 取 worktree HEAD 而非主仓库 HEAD；使用 `worktree_path` 字段判定
- 生命周期命令对 current-task 缺失的容错：`finish` 无 current-task 时只打印 `No current task set` 并 return 0；archive 要求显式传 `<task-name>`，不会误归档 current 任务

## 与其他模块的关系

| 模块 | 关系 |
|------|------|
| [kb-system](./kb-system.md) | archive 前校验 `kb_status`；本文档不复述 gate 细节 |
| [session-recording](./session-recording.md) | `finish` 触发 `_auto_record_session`；`finish`/`archive` 都触发 `refresh_global_workspace_index` |
| cli-commands | task 生命周期由 Python 脚本提供，非 Rust 二进制命令；二者为两套独立子命令系统 |
| file-management | task.json 的读写通过 `common.io.read_json/write_json`；目录迁移用 `shutil.move` |
| project-detection | `create --package` 会调用 monorepo 包校验 |
| template-system | `create` 生成的 prd.md 模板从 `kb/prd/_module-template.md` + 当前 `kb/prd/index.md` 的模块表拼装而来 |
