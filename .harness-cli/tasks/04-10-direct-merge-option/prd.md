# 并行任务直接合并分支（跳过 PR）

## Goal

支持两层"直接合并"能力：底层 pipeline 脚本支持 `direct-merge` action，上层 Claude command 支持 `--merge` 参数传递。解决小功能不需要 PR 审核流程的场景。

## Requirements

### Layer A：Pipeline 脚本层（底层能力）

#### A1. `start.py` 新增 `--merge` 参数

- `--merge [BRANCH]`：指定直接合并模式，BRANCH 默认为当前分支（base_branch）
- 写入 task.json：`"merge_mode": "direct"`, `"merge_target": "<branch>"`
- 修改 `next_action` 数组：最后一个 phase 的 action 从 `create-pr` 改为 `direct-merge`
- 不加 `--merge` 时行为完全不变

#### A2. 新增 `direct_merge.py` 脚本

位于 `.harness-cli/scripts/multi_agent/direct_merge.py`，与 `create_pr.py` 平行：

1. 读取 task.json 获取 merge_target
2. Stage + commit（复用 create_pr.py 的 commit 逻辑：排除 workspace、.agent-log 等）
3. 在 worktree 中：push feature 分支到 remote
4. 在主仓库中执行合并（worktree 中无法直接 checkout 其他分支）:
   - 方案：用 `git fetch origin feature/xxx && git merge --no-ff FETCH_HEAD` 在目标分支上操作
   - 或者：commit + push 后，用 `gh api` / 本地 git 操作在主仓库合并
5. 推送目标分支
6. 删除 remote feature 分支
7. 更新 task.json：status=completed, current_phase 设为 direct-merge 阶段

**合并冲突处理**：冲突时报错退出，打印清晰错误信息，保留 worktree 供手动解决。

#### A3. dispatch agent 识别 `direct-merge` action

在 `.claude/agents/dispatch.md` 中新增 `direct-merge` action 处理段：

```
### action: "direct-merge"

python3 ./.harness-cli/scripts/multi_agent/direct_merge.py
```

格式与现有 `create-pr` 一致。

#### A4. embedded 模板同步

以下文件需要同步更新：
- `embedded/templates/harness-cli/scripts/multi_agent/direct_merge.py`（新增）
- `embedded/templates/claude/agents/dispatch.md`（更新）
- 其他 AI 平台的 dispatch agent 如果有类似结构也要更新

### Layer B：Claude Command 编排层（上层 UX）

#### B1. 更新 `/hc:parallel` command

修改 `.claude/commands/hc/parallel.md` 和 `embedded/templates/claude/commands/hc/parallel.md`：

- 在"Step 4: Ask User for Requirements"中增加识别 `--merge` / `直接合并` 意图
- 在 start.py 调用时传递 `--merge <branch>` 参数
- 在"After Starting: Report Status"中根据 merge_mode 给出不同的提示信息

#### B2. 编排器后处理（可选增强）

当 merge_mode=direct 时，编排器检测到 agent 完成后：
- 自动从主仓库执行 `git pull` 拉取合并结果
- 自动调用 `cleanup.py -y` 清理 worktree
- 给出完成摘要（无 PR URL，改为显示合并的 commit hash）

## Acceptance Criteria

* [ ] `start.py --merge` 写入 task.json merge_mode + merge_target
* [ ] `start.py --merge master` 可指定目标分支
* [ ] task.json next_action 最后一个 phase 变为 `direct-merge`
* [ ] `direct_merge.py` 能 commit + merge 到目标分支
* [ ] 合并冲突时报错退出，不损坏仓库状态
* [ ] dispatch agent 能识别并执行 `direct-merge` action
* [ ] `/hc:parallel --merge` 在 Claude 对话中能正确传递参数
* [ ] 不加 `--merge` 时所有行为完全不变（向后兼容）
* [ ] embedded 模板与实例文件同步

## Definition of Done

* 代码风格与 start.py / create_pr.py 一致
* 不破坏现有 PR 流程
* dispatch.md 同时更新 embedded 和 .claude 版本

## Out of Scope

* 不修改 Rust CLI
* 不支持 squash merge（MVP 只做 --no-ff merge）
* 不支持交互式冲突解决

## Technical Notes

* `start.py`: `.harness-cli/scripts/multi_agent/start.py` — 入口，新增 --merge 参数
* `create_pr.py`: `.harness-cli/scripts/multi_agent/create_pr.py` — commit 逻辑可参考/复用
* `cleanup.py`: `.harness-cli/scripts/multi_agent/cleanup.py`
* `dispatch.md`: `.claude/agents/dispatch.md` — 新增 direct-merge action
* `parallel.md`: `.claude/commands/hc/parallel.md` — 编排层 UX
* task.json next_action 示例: `[{phase:1, action:"implement"}, ..., {phase:4, action:"direct-merge"}]`
* **worktree 限制**：worktree 中不能 checkout 其他分支，合并操作需要用 `git push` + 主仓库侧合并，或通过 `git merge` 技巧处理
