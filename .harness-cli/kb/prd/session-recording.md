# 会话记录系统

> 任务完成/归档时自动把开发记录写入开发者日志与工作空间索引，维护三层可追溯文件视图

## 模块概述

会话记录系统在任务完成或归档时自动把"发生了什么"沉淀到三层文件中：全局 `workspace/index.md`（Active Developers 表快照）、个人 `workspace/{dev}/index.md`（会话统计+历史）、个人 `workspace/{dev}/journal-N.md`（详细会话记录）。系统解决的核心问题是：之前的 `/hc:record-session` 手动记录从来没人跑，导致 journal 永远为空；本模块把记录从"pull-based 手动调用"改为"push-based 生命周期触发"（详见 `kb/tech/decisions.md` ADR-015）。

## 关键文件

| 文件路径 | 职责 |
|----------|------|
| `.harness-cli/scripts/add_session.py` | 两种会话添加模式：手动 (`--title ...`) 和自动 (`--from-task <task.json>`)；写 journal、更新个人 index、可选 auto-commit |
| `.harness-cli/scripts/update_workspace_index.py` | 全局 workspace/index.md 的 Active Developers 表刷新；扫描所有 `workspace/{dev}/` 子目录汇总状态与活跃时间 |
| `.harness-cli/scripts/common/task_utils.py` | `refresh_global_workspace_index(repo_root)` —— 全局刷新的统一入口（非阻塞包装，供 task 生命周期调用） |
| `.harness-cli/scripts/task.py` `_auto_record_session()` | `finish` 命令的 orchestrator：两步（session 记录 + 全局刷新），相互隔离、都非阻塞 |
| `.harness-cli/scripts/common/developer.py` | 开发者初始化：创建 `.developer`、`workspace/{dev}/` 目录、初始 `journal-1.md` 与 `index.md` 模板 |
| `embedded/templates/markdown/workspace-index.md` | 全局索引模板源（简化版：Active Developers 表 + 目录结构说明） |
| `.harness-cli/config.yaml` | 配置：`session_commit_message`（auto-commit 的消息）与 `max_journal_lines`（journal 轮转阈值，默认 2000） |

## 核心功能

### 1. 三层文件视图

| 层级 | 路径 | 职责 | 维护者 |
|------|------|------|--------|
| 全局 | `.harness-cli/workspace/index.md` | 团队视角：每个开发者 + 活跃任务 + 状态汇总 + 最后活跃日期 | `update_workspace_index.py` |
| 个人 | `.harness-cli/workspace/{dev}/index.md` | 个人视角：当前活跃 journal、Total Sessions、Session History 表 | `add_session.py` `update_index()` |
| 日志 | `.harness-cli/workspace/{dev}/journal-N.md` | 详细会话：每个 session 包含标题、日期、分支、commits、Main Changes 等 | `add_session.py` `generate_session_content()` |

### 2. `_auto_record_session` —— finish 时的 orchestrator

**物理位置**：`.harness-cli/scripts/task.py`（并非 task_store.py）

**设计**：两步副作用，相互独立的 try/except，一处失败不影响另一处。

```
_auto_record_session(task_json_path, repo_root)
  ├── Step 1: 写 journal + 个人 index
  │   try:
  │     add_session_from_task(task_json_path, auto_commit=False)
  │   except (Exception, SystemExit) as e:
  │     # SystemExit 必须单独 catch——ensure_developer 未初始化时会 sys.exit
  │     打印 ⚠ session recording failed
  │
  └── Step 2: 刷全局 index
      refresh_global_workspace_index(repo_root)
      # 内部本身已经 try/except (Exception, SystemExit)
```

**调用点**：
- `cmd_finish`（task.py）中，在 `_finalize_task_on_finish` 之后、`clear_current_task` 之前调用
- `cmd_archive`（task_store.py）在归档结束后**只**调用 `refresh_global_workspace_index`（session 已在 finish 时记录过）

**路径 bootstrap**：该函数会懒加载 sibling scripts `add_session` 模块，为此需要把 scripts 目录注入 `sys.path`（做了去重检查避免重复 insert）。

### 3. `add_session_from_task` —— 自动模式

- **业务规则**：从 task.json 自动抽取 title / branch / commits / summary，无需用户参数
- **触发条件**：`python3 add_session.py --from-task <path>` 或 `_auto_record_session` 内部调用
- **关键字段抽取顺序**：

| 字段 | 优先级（从高到低） |
|------|---------------------|
| title | `task.json.title` → `task.json.name` → `"unknown task"` |
| branch | `task.json.branch` → `git branch --show-current` |
| commits | `git log <base>..<branch> --oneline --format=%h`；`<base>` 按 `task.json.base_branch` → `main` → `master` 三级回退 |
| summary | `task.json.description`（非空优先）→ prd.md 首段（H1 标题后、首个 `##` 前的非空行） → `"(Auto-recorded session)"` |

- **developer 预检**：函数起始就通过 `get_developer()` 预检开发者是否已初始化，避免 `ensure_developer()` 里 `sys.exit()` 冒泡
- **package 自动解析**：从 task.json 读 `package` 字段直接传给 `add_session`
- **auto_commit 默认 False**：finish 调用时不 auto-commit（避免 `finish` 产生额外 commit）

### 4. `add_session` —— 手动模式

- **业务规则**：用户显式传入 `--title --commit --summary` 追加一条 session
- **触发条件**：`python3 add_session.py --title "..." [--commit h1,h2] [--summary ...] [--package pkg] [--branch name] [--content-file path | --stdin]`
- **分支推断链**：CLI `--branch` → `task.json.branch`（当前任务） → `git branch --show-current`
- **package 推断链**：CLI `--package`（monorepo 外时忽略） → 当前任务 `task.json.package` → `default_package` 配置
- **auto_commit 默认 True**：CLI 手动模式默认会 auto-commit `workspace/` + `tasks/` 目录（可用 `--no-commit` 关闭）

> **注意两种模式 auto_commit 默认值相反**：finish 自动路径 `auto_commit=False`，CLI 手动路径 `auto_commit=(not args.no_commit)=True`。这是刻意设计——`finish` 路径自身已在 hooks 中让外部流程决定提交时机；CLI 路径假设用户希望"一键记录并提交"。

### 5. Journal 轮转

- **阈值**：`config.yaml` 的 `max_journal_lines`（默认 2000）
- **判定**：`current_lines + new_session_lines > max_lines` 时触发轮转
- **轮转动作**：创建 `journal-{N+1}.md`，文件头记录来自 `journal-N.md` 的 continuation 注释
- **编号规则**：从 1 起，基于 `get_latest_journal_info()` 扫描现有文件的数字后缀取最大值
- **老 journal 仍保留**：不会合并/压缩，只是后续 session 写入新文件

### 6. 个人 `index.md` 更新（基于 marker 的幂等替换）

**触发**：每次 `add_session()` 成功后自动调用 `update_index()`。

**定位机制**：通过三对 HTML 注释 marker 标识可替换区域：

| Marker | 区域内容 |
|--------|---------|
| `@@@auto:current-status` | Active File / Total Sessions / Last Active 三行 |
| `@@@auto:active-documents` | 所有 journal 文件的行数+状态表 |
| `@@@auto:session-history` | 按编号倒序的 Session 列表（# / Date / Title / Commits / Branch） |

**历史列迁移**：`update_index` 会把旧的 4 列/6 列 Session History 表头自动迁移为现行 5 列（Branch-only）格式。

**幂等**：多次运行结果一致。

### 7. 全局 `workspace/index.md` 刷新

**入口**：`update_workspace_index.update_workspace_index(repo_root)`

**被调用**（通过 `refresh_global_workspace_index` 包装）：
- `cmd_finish` 流程末尾（经由 `_auto_record_session` Step 2）
- `cmd_archive` 归档成功后

**刷新内容**：Active Developers 表（marker `@@@auto:developers`）

```
| Developer | Current Tasks | Status | Last Active |
```

- **Developer 列**：扫描 `workspace/{dev}/` 子目录得到开发者列表
- **Current Tasks 列**：`iter_active_tasks` 过滤 `assignee == dev` 的任务 title
- **Status 列**：`_format_status_summary` 按生命周期顺序排列（`planning → in_progress → review → completed`），未知状态字母序后缀
- **Last Active 列**：开发者目录下所有 `journal-N.md` 的最新 mtime（YYYY-MM-DD 形式）

**幂等**：基于 marker 替换，多次运行产生相同结果。

### 8. 开发者初始化

- **触发**：`add_session.py` 首次调用时通过 `ensure_developer()` 自动执行，或用户显式运行 `init_developer.py`
- **创建**（若不存在）：
  1. `.harness-cli/.developer`（记录开发者名 + 初始化时间戳）
  2. `.harness-cli/workspace/{dev}/` 目录
  3. `journal-1.md`（含标题、起始日期、分隔符）
  4. `index.md`（含四个 marker 区段，初始 Total Sessions=0）
- **幂等**：已初始化时跳过所有创建步骤

### 9. auto-commit workspace 变更

- **执行**：`_auto_commit_workspace(repo_root)` 在 `add_session` 末尾按 `auto_commit` 参数决定调用
- **动作**：`git add -A .harness-cli/workspace .harness-cli/tasks` → 检查是否有 staged 改动 → `git commit -m "<session_commit_message>"`
- **默认 commit message**：`chore: record journal`（可在 `config.yaml.session_commit_message` 自定义）
- **提交内容**：包含 journal 更新 + 个人 index 更新；**不**包含全局 `workspace/index.md`（它在 `refresh_global_workspace_index` 执行时更新，但不会被此处的 `git add` 捕获——全局 index 的提交通常由 archive 的 auto-commit 一并处理）

## 数据流

```
┌─── task.py finish ───────────────────────────────────────────┐
│                                                               │
│  _finalize_task_on_finish()  → task.json 写完成字段           │
│          │                                                    │
│          ↓                                                    │
│  _auto_record_session(task_json_path, repo_root)              │
│   ├─ Step 1:                                                  │
│   │    add_session_from_task(task_json, auto_commit=False)    │
│   │      ├─ 抽取 title/branch/commits/summary                 │
│   │      ├─ get_latest_journal_info() → 计算是否要轮转        │
│   │      ├─ append session 段到 journal-N.md                  │
│   │      └─ update_index() 按 marker 替换个人 index           │
│   │                                                           │
│   └─ Step 2:                                                  │
│        refresh_global_workspace_index(repo_root)              │
│          → _build_developers_table() 扫描所有 developer       │
│          → 替换 workspace/index.md 的 @@@auto:developers 区段 │
│                                                               │
│  clear_current_task()                                         │
│  after_finish hook                                            │
└───────────────────────────────────────────────────────────────┘

┌─── task.py archive ──────────────────────────────────────────┐
│  (略去 KB gate / 目录迁移 / auto-commit 等部分)               │
│  refresh_global_workspace_index(repo_root)                    │
│   → 已归档任务不再出现在 Active Developers 表                 │
└───────────────────────────────────────────────────────────────┘

┌─── python3 add_session.py --title ... (CLI 手动) ────────────┐
│  add_session(title, commit, summary, auto_commit=True)        │
│   ├─ 写 journal + update_index                                │
│   └─ _auto_commit_workspace() → git commit                    │
└───────────────────────────────────────────────────────────────┘
```

## 业务规则

- **非阻塞**：session 记录和全局索引刷新都是"best-effort"副作用，任何失败只打印 `[WARN]` 不中断 task lifecycle
- **幂等**：全局索引和个人索引都基于 marker 注释替换，多次运行产生相同结果
- **SystemExit 必须被捕获**：`ensure_developer()` 用 `sys.exit(1)` 报告未初始化；session 记录代码必须 catch `SystemExit` 否则会炸穿 `finish`
- **Summary 提取链**：task.json 的 `description`（非空优先）→ prd.md 首段 → `"(Auto-recorded session)"`
- **Commits 提取链**：`git log <base>..<branch>`，base 候选 `task.json.base_branch` → `main` → `master`；若以上全部失败，回退单个 `task.json.commit[:8]`；仍失败则显示 `-` 或 `(No commits - planning session)`
- **auto_commit 两种默认值**：finish 路径默认 `False`；CLI 手动路径默认 `True`（意图不同）
- **Last Active 基于 mtime**：全局表的"最后活跃"时间由 journal 文件 mtime 推断（非完美——`git clone` 后 mtime 被重置；该已知限制留待未来改进）
- **Max journal lines 可配置**：默认 2000；达到阈值时创建下一个编号的 journal 文件，老 journal 不归档也不合并
- **开发者隔离**：每个开发者一个 `workspace/{dev}/` 子目录，互不影响；全局表只读不写个人数据
- **状态汇总顺序固定**：Active Developers 表的 Status 列按生命周期排列，保证团队视角的可读性一致

## 与其他模块的关系

| 模块 | 关系 |
|------|------|
| [task-lifecycle](./task-lifecycle.md) | `finish` 和 `archive` 是本模块的触发点；本模块拥有 `_auto_record_session` 的完整流程叙事 |
| [kb-system](./kb-system.md) | KB gate 与 session 记录相互独立——archive 流程先过 KB gate 再刷全局索引（已归档任务不会出现在 Active Developers 表） |
| file-management | workspace 下所有文件的写入通过标准 Python 文件 I/O；auto-commit 通过 subprocess + git 命令 |
| template-system | `workspace/index.md` 的初始模板来自 `embedded/templates/markdown/workspace-index.md`（详见 template-system.md）；个人 `index.md` 模板内嵌在 `developer.py` |
| project-detection | 与本模块无直接关系（session 内容不依赖项目类型） |
