# 修复 task.json 状态字段异常

## Goal

修复 `.harness-cli/tasks/` 下 task.json 普遍存在的字段脏数据问题：`current_phase` 停留在 0、`status` 卡在 `planning`、`dev_type` 为 `None`、`completedAt` / `commit` 空置，以及老架构任务缺字段。定位根因，决定是"只回填脏数据"还是"同时修正写入路径避免复发"。

## What I already know (调研结论)

### 观察到的脏数据

| 任务 | status | current_phase | dev_type | completedAt | 问题 |
|---|---|---|---|---|---|
| `00-bootstrap-guidelines` (active) | `in_progress` | **字段缺失** | `docs` | null | 旧 schema，完全缺 `current_phase`/`next_action`/`branch`/`base_branch`/`worktree_path`/`scope`/`package`/`pr_url` |
| `04-10-04-10-update-kb` (active) | `in_progress` | `0` | backend | null | phase 未推进 |
| `04-10-kb-tech-layer-v2` (active) | `in_progress` | `0` | backend | null | phase 未推进 |
| `archive/04-10-workflow-modes` (completed) | `completed` | **`0`** | backend | 2026-04-11 | 已完成却仍在 phase 0 |
| `archive/04-10-workspace-index-enhancement` (completed) | `completed` | **`0`** | backend | 2026-04-11 | 同上 |
| `archive/04-10-direct-merge-option` | **`planning`** | `0` | backend | null | 已归档却仍为 `planning` |
| `archive/04-10-kb-tech-layer` | **`planning`** | `0` | backend | null | 同上 |
| `archive/04-10-task-status-command` | **`planning`** | `0` | backend | null | 同上 |
| `archive/04-10-add-scan-kb-command` (completed) | `completed` | `0` | **`None`** | 2026-04-10 | 从未跑 `init-context`，dev_type 为空 |
| 多数 archived tasks | `completed` | `0` | `None` | 有值 | 同上模式 |
| **所有任务** | — | — | — | — | `commit` 字段从未被回填 |

### 根因（代码层面）

1. **Schema 漂移**
   - `scripts/create_bootstrap.py` 创建的 bootstrap 任务用的是**旧 schema**，不含 pipeline 字段。
   - `scripts/common/task_store.py:cmd_create` 创建的是**新 schema**。两套 schema 并存。

2. **`cmd_start` 不回写 task.json** (`task.py:70`)
   - 只写 `.current-task` 并调用 hook，不会把 `status: planning → in_progress`。
   - 结果：用户开始干活以后，task.json 仍然显示 `planning`。

3. **`cmd_finish` 不回写 task.json** (`task.py:106`)
   - 只清 `.current-task`，不更新 `status`、`completedAt`、`current_phase`、`commit`。

4. **`current_phase` 只在 multi_agent 管线里推进**
   - `common/phase.py` 提供 `set_phase` / `advance_phase`，但只有 `multi_agent/create_pr.py` 和 `multi_agent/direct_merge.py` 调用（task_store.py:232 初始化为 0）。
   - 普通 dev 模式（非 multi_agent）下 `current_phase` 永远停在 0，即使已完成归档。

5. **`cmd_archive` 只打半边补丁** (`task_store.py:295`)
   - 归档时设置 `status=completed` 和 `completedAt`，但**不**把 `current_phase` 推到终态，也不回填 `commit`。
   - 所以已归档任务全是 `completed + phase 0` 的矛盾状态。
   - 归档目录里那些 `status=planning` 的任务，很可能是**直接 `mv` 进 archive** 没走 `cmd_archive`。

6. **`init-context` 确实会回写 `dev_type` 和 `package`**（`task_context.py:190`）
   - 所以 `dev_type: None` 的任务是根本没跑过 `init-context`，不是代码 bug。

### 涉及的消费方

读 `status` 的地方很多：`session_context.py`（看板、MY TASKS）、`iter_active_tasks`、`multi_agent/status_display.py`。读 `current_phase` 的地方少：主要是 `phase.get_phase_info` 被 `status_display.py` 调用。`commit` 字段只在 `multi_agent/create_pr.py` 里被消费。

## Assumptions (待用户确认)

- 用户日常走 **dev 模式 + archive**，很少走 multi_agent 管线，所以脏数据是长期积累
- 主要痛点是看板 / dashboard 上数据显得"乱"，以及 `00-bootstrap-guidelines` 的缺字段可能导致脚本崩
- 用户希望"修完之后不再复发"，而不是只打扫一次卫生

## Decisions (ADR-lite)

### D1：修复范围 = 修正写入路径 + 回填历史（2026-04-11）

- **Context**：脏数据一方面来自历史积累，一方面来自代码漏洞会继续产生
- **Decision**：先改代码，再跑迁移脚本一次性对齐
- **Consequences**：工作量相对大，但彻底解决问题；需要确保迁移脚本幂等

### D2：状态推进规则 = 极简三态（2026-04-11）

- **Context**：dev 模式的实际使用路径是 `create → start → 干活 → finish → archive`
- **Decision**：`current_phase` 只有 0 / 1 / 终态 三种。`finish` 做完整收尾（`status=completed`、`completedAt`、`commit=HEAD`、`current_phase=终态`），`archive` 纯粹负责 mv 目录 + 兜底补缺
- **Consequences**：改动面小，不需要改 hook；dev 模式里 `current_phase` 丢失 implement/check 中间态，但 multi_agent 管线不受影响

## Open Questions

- **Q3**：旧 schema 任务（如 `00-bootstrap-guidelines`）如何处理？
- **Q4**：归档目录里那些 `status=planning` 的任务是不是手工 `mv` 进去的？迁移脚本要不要统一补 `completed`？

## Requirements (evolving)

- [x] 定位并记录所有脏字段的产生路径（写在上面）
- [x] 设计迁移脚本回填历史 task.json（`scripts/migrate_task_json.py`，带 `--apply`）
- [x] 修正 `cmd_start` / `cmd_finish` / `cmd_archive` 的字段推进逻辑
- [x] 统一新旧 schema（`create_bootstrap.py` 已对齐 `task_store.cmd_create`）

## Acceptance Criteria (evolving)

- [x] 运行迁移脚本后，所有 archived task 的 `status=completed` 且 `current_phase` 处于终态
- [x] 所有 active task 的 `status` 与 `.current-task` 一致（`cmd_start` 会把 planning → in_progress）
- [x] `00-bootstrap-guidelines` 拥有完整的新 schema 字段
- [x] 从新创建到归档的完整生命周期内，task.json 字段自动保持正确（smoke 测试双路径验证通过）
- [x] 现有命令（`list`、`status`、dashboard）不会因 schema 升级而崩

## Definition of Done

- 迁移脚本附带 dry-run 模式
- Lint / typecheck 通过
- 文档更新（spec 或 workflow.md 说明状态生命周期）
- 归档前后的 task.json 字段有明确的规范表

## Out of Scope (explicit)

- 不重新设计 phase 模型本身（复用现有 4 阶段）
- 不动 multi_agent 管线里的 phase 推进逻辑
- 不迁移 `.harness-cli/kb/` 下的文档

## Technical Notes

- 关键文件：
  - `.harness-cli/scripts/common/task_store.py` (`cmd_create`, `cmd_archive`)
  - `.harness-cli/scripts/task.py` (`cmd_start`, `cmd_finish`)
  - `.harness-cli/scripts/common/phase.py`
  - `.harness-cli/scripts/common/task_context.py` (`cmd_init_context` 已有回写逻辑可参考)
  - `.harness-cli/scripts/create_bootstrap.py` (旧 schema 源头)
- 参考字段清单见 `common/types.py:TaskData`
