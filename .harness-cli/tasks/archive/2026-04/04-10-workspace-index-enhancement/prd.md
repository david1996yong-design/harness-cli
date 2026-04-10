# workspace/index.md 展示开发者及职责

## Goal

让 `workspace/index.md` 自动展示所有参与开发的人及其负责的内容（当前任务），替代永远显示 `(none yet)` 的静态模板。

## Requirements

### 1. 新增 `update_workspace_index.py` 脚本

位于 `.harness-cli/scripts/update_workspace_index.py`：

- 扫描 `workspace/` 下所有开发者子目录（排除 index.md 自身）
- 对每个开发者，从 `tasks/` 目录中查找 assignee 匹配的活跃任务
- 生成 Active Developers 表，包含列：
  - Developer（名称）
  - Current Tasks（当前分配的任务标题，多个用逗号分隔）
  - Status（任务状态汇总：如 "2 in_progress, 1 planning"）
  - Last Active（从 journal 文件的修改时间推断）
- 用生成的表替换 `workspace/index.md` 中 `## Active Developers` 下的表格
- 使用标记注释（如 `<!-- @@@auto:developers -->` ... `<!-- @@@/auto:developers -->`）定位替换区域
- 脚本幂等：多次运行结果一致

### 2. 更新 workspace/index.md 模板

修改 `embedded/templates/markdown/workspace-index.md`：

- Active Developers 表替换为带自动更新标记的版本：
  ```markdown
  ## Active Developers

  <!-- @@@auto:developers -->
  | Developer | Current Tasks | Status | Last Active |
  |-----------|--------------|--------|-------------|
  | (run update_workspace_index.py to populate) | - | - | - |
  <!-- @@@/auto:developers -->
  ```
- 在 Getting Started 中增加提示：运行 `python3 .harness-cli/scripts/update_workspace_index.py` 刷新索引

### 3. 在 `init_developer.py` 中触发更新

`init_developer.py` 创建新开发者后，自动调用 `update_workspace_index.py` 刷新全局索引。

### 4. embedded 模板同步

- `embedded/templates/harness-cli/scripts/update_workspace_index.py`（新增）
- `embedded/templates/markdown/workspace-index.md`（更新）

## Acceptance Criteria

* [ ] `update_workspace_index.py` 存在且可运行
* [ ] 运行后 `workspace/index.md` 的 Active Developers 表显示所有开发者
* [ ] 每个开发者行显示其当前分配的任务
* [ ] `init_developer.py` 创建新开发者后自动刷新索引
* [ ] embedded 模板同步更新
* [ ] 脚本幂等，多次运行结果一致

## Definition of Done

* Python 代码风格与现有 scripts 一致（使用 common/ 工具）
* 中文注释

## Out of Scope

* 不修改 Rust CLI
* 不修改个人 workspace/{name}/index.md（只改全局 index）

## Technical Notes

* 全局 index: `workspace/index.md` — 模板来自 `embedded/templates/markdown/workspace-index.md`
* 模板访问器: `src/templates/markdown.rs` 中的 `agent_progress_index_content`
* init_developer: `.harness-cli/scripts/init_developer.py` + `.harness-cli/scripts/common/developer.py`
* 任务数据: `.harness-cli/scripts/common/tasks.py` — `iter_active_tasks`
* 任务队列: `.harness-cli/scripts/common/task_queue.py` — `list_tasks_by_assignee`
* 个人 index 已有标记系统: `<!-- @@@auto:xxx -->` 模式（参考 `developer.py:95`）
