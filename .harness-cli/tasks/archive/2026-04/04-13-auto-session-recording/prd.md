# 自动化 Session Recording

## Goal

在 `task.py finish` 时自动调用 session recording，将任务信息写入 journal 和 index.md，无需用户手动运行 `/hc:record-session`。

## Background

当前 session recording 链路是断裂的：
- `add_session.py` 需要手动触发，没人会主动去跑
- 导致 `workspace/{dev}/index.md` 和 `journal-*.md` 永远是空的
- `workspace/index.md` 的 "Getting Started" 指引也过时无用

## Requirements

### 1. `add_session.py` 新增 `--from-task` 模式
- 接受 task.json 路径作为参数
- 自动从 task.json 提取：title, branch, commit, description
- 自动从 git log 提取该任务的 commits（`git log main..{branch} --oneline`）
- 如果没有 branch 信息，fallback 到 task.json 的 `commit` 字段

### 2. `task.py cmd_finish()` 自动调用 session recording
- 在 `_finalize_task_on_finish()` 完成后、`clear_current_task()` 之前
- 直接 Python 函数调用 `add_session()`，不走 hook
- 失败不阻塞 finish 流程（catch exception，打印 warning）

### 3. 简化 `workspace/index.md` 模板
- 去掉无用的 "Getting Started" 和 "For New/Returning Developers" 指引
- 保留 Active Developers 表（由 `update_workspace_index.py` 维护）

## Acceptance Criteria

- [ ] `task.py finish` 后 journal 文件自动追加 session 记录
- [ ] `workspace/{dev}/index.md` 的 session count 和 history 表自动更新
- [ ] 手动 `add_session.py --title ...` 原有用法不受影响
- [ ] session recording 失败时 finish 仍然正常完成
- [ ] git commits 能从 branch 信息自动提取

## Technical Notes

- 注入点：`cmd_finish()` 内部，Python 函数级调用
- 不用 hook 机制：因为需要在 `clear_current_task()` 之前执行（task.json 还在原位）
- `add_session.py` 的 `auto_commit` 参数设为 False（finish 场景不需要额外 git commit）
