# Archive 阻塞检查：task 未声明 KB 状态时禁止归档

## Goal

Archive 时强制要求 task 显式声明 KB 状态（已更新 / 不需要更新），避免用户在业务逻辑变更后遗忘更新 KB。判断"是否需要更新"交给 AI 模型，不做基于文件路径的自动白名单。

## Background

目前 `after_archive` hook 只是 `echo` 一条提醒，容易被忽略。需要改为强制阻塞：
- `kb_status` 字段记录任务的 KB 处理状态
- Archive 时 `needed` 状态被 block
- 由 AI 在 finish/scan-kb 环节判断并设置状态

## Requirements

### 1. task.json 新增 `kb_status` 字段
- 枚举值：`"needed"` | `"updated"` | `"not_required"`
- 新建任务默认 `"needed"`
- 读取老任务时，字段缺失按 `"needed"` 兜底

### 2. 新命令 `task.py mark-kb`
```
python3 task.py mark-kb <status> [<task-name>]
  status: needed | updated | not-required
  task-name: 可选，不传则使用当前 current task
```
- 更新 task.json 的 kb_status 字段
- 验证 status 合法性

### 3. `cmd_archive` 增加 KB 状态检查
- 读取 task.json 的 `kb_status`
- 如果是 `"needed"` → block（返回 1），打印清晰的错误信息指引：
  - 如果涉及业务逻辑：运行 `/hc:scan-kb` 更新 KB
  - 如果不涉及：运行 `task.py mark-kb not-required <task>`
- `"updated"` 或 `"not_required"` → 放行

### 4. 不加逃生阀
- 不提供 `--force` 或 `--skip-kb-check` 这种 flag
- 用户判断不需要 KB 时走 `mark-kb not-required` 流程

## Acceptance Criteria

- [ ] 新建 task 时 task.json 自带 `kb_status: "needed"`
- [ ] `task.py mark-kb not-required <task>` 能正确更新字段
- [ ] `task.py mark-kb` 传入非法状态时报错
- [ ] `task.py archive` 对 `kb_status=needed` 的任务报错并返回 1
- [ ] `task.py archive` 对 `kb_status=updated/not_required` 的任务正常归档
- [ ] 错误信息清晰，给出两条操作路径
- [ ] 老任务（没有 kb_status 字段）的 archive 被 block（兜底为 needed）
- [ ] 131 Rust 回归测试仍然通过

## Technical Notes

- 改动点：
  - `task.py`：新增 `cmd_mark_kb` 和 CLI 子命令
  - `common/task_store.py`：`cmd_create` 加默认值、`cmd_archive` 加检查
- 不改：
  - `/hc:scan-kb` 的 command 文档暂不改（属于 AI workflow 层，可后续 task）
  - `/hc:finish-work` 的 workflow 提示暂不改
- 迁移策略：老任务通过 `.get("kb_status", "needed")` 兜底，不做批量迁移
