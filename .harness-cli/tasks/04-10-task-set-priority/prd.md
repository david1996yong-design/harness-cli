# task.py 新增 set-priority 子命令

## Goal

为 `task.py` 增加 `set-priority` 子命令，允许修改已有任务的优先级。
当前只能在 `create` 时通过 `--priority` 设置，无法事后修改。

## Requirements

- 新增 `set-priority` 子命令，用法：`python3 task.py set-priority <task-dir> <P0|P1|P2|P3>`
- 修改 `task.json` 中的 `priority` 字段
- 参数校验：优先级必须是 P0/P1/P2/P3 之一，否则报错
- 输出确认信息，如：`✓ Priority set to: P1`
- 参考现有 `set-branch`、`set-scope` 的实现模式保持一致

## Acceptance Criteria

- [ ] `python3 task.py set-priority .harness-cli/tasks/04-10-xxx P1` 正确修改 task.json
- [ ] 无效优先级（如 P5）报错退出
- [ ] 不存在的任务目录报错退出
- [ ] `python3 task.py --help` 中显示 set-priority 子命令

## Technical Notes

- 改动文件：`.harness-cli/scripts/task.py`
- 参考模式：`set-branch` 和 `set-scope` 子命令的实现（argparse + read/write task.json）
