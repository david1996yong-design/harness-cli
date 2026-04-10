# 查看所有任务状态的命令

## Goal

提供三层任务状态查看能力：增强现有 list 命令的详细输出、新增 status 仪表盘子命令、新增 Claude 命令 `/hc:task-dashboard`，让开发者在不同场景下都能快速了解所有任务的完整状态。

## Requirements

### Feature 1: 增强 `task.py list --detail`

在现有 `task.py list` 基础上增加 `--detail` / `-d` 标志，输出更丰富的信息：

- 保持现有简略输出为默认行为（不破坏兼容性）
- `--detail` 模式下每个任务显示：
  - priority（P0-P3）
  - title（人类可读标题）
  - status
  - assignee
  - branch（如果有）
  - 创建时间（createdAt）
  - package（如果有）
  - children progress（如果有子任务）
- 格式：每个任务多行缩进展示，或使用对齐的表格

### Feature 2: 新增 `task.py status` 子命令

专门的状态仪表盘，一次展示全貌：

- 优先级统计：P0/P1/P2/P3 计数（已有 `get_task_stats`）
- 按状态分组显示：planning → in_progress → completed
- 每个任务显示：priority、title、assignee、branch
- 支持 `--mine` 过滤
- 支持 `--json` 输出 JSON 格式（方便脚本消费）
- 底部汇总行：总数、各状态计数

### Feature 3: 新增 Claude 命令 `/hc:task-dashboard`

创建 `.claude/commands/hc/task-dashboard.md` 和 `embedded/templates/claude/commands/hc/task-dashboard.md`：

- 调用 `python3 task.py status --json` 获取数据
- 以 markdown 表格 + 摘要形式展示
- 包含智能建议：如有 P0 任务未处理则提醒，如有长期 in_progress 任务则提醒
- 显示 agent 运行状态（读取 registry.json）

## Acceptance Criteria

* [ ] `task.py list --detail` 显示每个任务的 priority、title、status、assignee、branch、createdAt
* [ ] `task.py list` 不加 `--detail` 时输出保持不变
* [ ] `task.py status` 按状态分组显示所有任务
* [ ] `task.py status` 底部显示优先级统计和状态计数
* [ ] `task.py status --json` 输出合法 JSON
* [ ] `task.py status --mine` 只显示当前开发者的任务
* [ ] `/hc:task-dashboard` Claude command 文件存在于两个位置
* [ ] Claude command 调用 `task.py status --json` 并格式化输出

## Definition of Done

* 现有 `task.py list` 的默认行为不变
* 新增的代码风格与现有 task.py 一致
* 同时更新 embedded 模板和实例文件（Claude command）

## Out of Scope

* 不修改 Rust CLI 代码
* 不修改 `multi_agent/status.py`（它有独立的 agent 状态功能）
* 不添加其他 AI 平台的对应命令

## Technical Notes

* `task.py` cmd_list: `.harness-cli/scripts/task.py:128` — 增强此函数
* 新增 cmd_status 函数到 task.py
* TaskInfo 定义: `.harness-cli/scripts/common/types.py:60`
* task_queue.py 已有 `get_task_stats`、`list_tasks_by_status` — 可直接复用
* iter_active_tasks: `.harness-cli/scripts/common/tasks.py:54`
* Claude command 参考格式: `embedded/templates/claude/commands/hc/scan-kb.md`
* embedded 和 .claude 目录需要同步放置 command 文件
