# 周报系统

> 每周聚合开发者本地数据（任务 / commits / journal / KB 变更），生成"给自己看"的 Markdown 周报

## 模块概述

周报系统是个人级的 push-based 回顾工具。它不调用 AI、不推送外部系统、不依赖网络，完全从本地可得数据生成确定性事实区；AI 洞察由独立的 `/hc:weekly-review` slash command 在锚点下方追加，二者解耦。v1 只做个人版，不做团队聚合或汇报版。

## 关键文件

| 文件路径 | 职责 |
|----------|------|
| `.harness-cli/scripts/common/weekly_report.py` | 核心模块：ISO 周解析、任务/commits/journal/KB 聚合、Markdown 渲染、Sun/Mon 提示判定 |
| `.harness-cli/scripts/task.py` | 注册 `weekly-report` 子命令 → `cmd_weekly_report`，委托给 `generate_weekly_report()` |
| `.harness-cli/scripts/common/session_context.py` | 在 `get_context_text` 末尾调用 `should_remind_weekly_report()`，满足条件时追加 `## WEEKLY REPORT` 段 |
| `.claude/commands/hc/weekly-review.md` | AI 总结 slash command：读取最近一份周报，在 `## AI 总结` 锚点下方生成"亮点 / 阻塞 / 下周建议"三段 |

## 核心功能

### 周报生成

- **业务规则**：基于 ISO 周（周一~周日），按开发者身份聚合当周事实；幂等覆盖
- **触发条件**：`python3 .harness-cli/scripts/task.py weekly-report [--week YYYY-Www] [--dev <name>]`
- **处理流程**：
  1. 校验 `--dev` 值（拒绝路径穿越、`.` 开头、包含 `/` `\\` `..`）
  2. 解析 ISO 周为 (year, week, monday, sunday)
  3. 聚合四类事实：
     - **任务**：扫 `tasks/archive/YYYY-MM/` 按 `completedAt` 落在周内；扫 `tasks/` 活跃任务按 `createdAt`；历史周不显示"当下 in_progress"
     - **Commits**：`git log -E --author='<user.name>|<user.email>'` 按天分组（优先用 git config 身份，fallback 到 `.developer`）
     - **Journal**：扫 `workspace/{dev}/journal-*.md` 的 `^## ` 标题配合 `**Date**: YYYY-MM-DD` 行
     - **KB/spec 变更**：`git log --name-only` 过滤 `kb/prd/`、`kb/tech/`、`spec/` 路径
  4. 渲染 Markdown，末尾固定放 `## AI 总结` 锚点 + 占位符
  5. 写入 `workspace/{dev}/reports/{YYYY}-W{NN}.md`（幂等覆盖）

### Session-start 提示

- **业务规则**：周日或周一首次会话，若目标周的报告缺失，则在 `SESSION CONTEXT` 追加一行提示
- **触发条件**：`get_context_text()` 每次执行时检查日历
- **处理流程**：
  - 周日（weekday=7）→ 目标周 = 当前周（本周快结束，提醒生成本周 retro）
  - 周一（weekday=1）→ 目标周 = 上一周（上周刚结束，提醒回顾）
  - 周二到周六 → 静默
  - 检查 `reports/{target}.md` 是否存在；不存在才提示
  - 提示消息中包含具体周号与可复制命令

### AI 总结追加

- **业务规则**：`/hc:weekly-review` 只在 `## AI 总结` 锚点**下方**写入三段内容；事实区一字节不改
- **触发条件**：用户主动执行 `/hc:weekly-review`
- **处理流程**：定位本周 `reports/*.md` → 读事实区 → 生成"亮点 / 阻塞 / 下周建议" → 替换占位符；若已有 AI 区则整段替换、不累加

## 数据流

```
tasks/ + tasks/archive/       ┐
git log (commits + diff stat) ├─→ collect_* ─→ render_report ─→ reports/{YYYY}-W{NN}.md
workspace/{dev}/journal-*.md  ┘                                        │
                                                                       ▼
                                                    /hc:weekly-review ─→ AI 区追加
```

## 业务规则

- **周定义**：ISO 8601 周（周一起算，周日结束）
- **幂等**：同一周重复运行，覆盖同一文件；不产生 `-1` / `-2` 副本
- **历史周 vs 当前周**：查询历史周时，当下 `in_progress` 但非当周创建的任务**不出现**；当前周则包含（用于持续性任务的状态感知）
- **事实与 AI 解耦**：脚本只写确定性事实；AI 总结由独立 slash command 追加。AI 不在场时功能不退化
- **默认排除项**（v1 刻意不做）：邮件/Slack/webhook 推送、代码行数与工时、效率打分、团队聚合、HTML Dashboard、跨项目聚合

## 安全与健壮性

- **--dev 校验**：拒绝 `/`、`\\`、`..`、以 `.` 开头的值，防止写入 workspace 外
- **Git author 来源**：优先 `git config user.name / user.email`（字面转义后以 `-E` 扩展正则 `|` 拼接）；仅在两者都未配置时回退到 `.developer` 名
- **空周友好**：无任务、无 commit、无 journal 时渲染"本周安静"，不报错
- **session-start 非阻塞**：提示注入失败被静默捕获，不影响上下文生成

## 与其他模块的关系

| 模块 | 关系 |
|------|------|
| task-lifecycle | 读取 `tasks/` 和 `tasks/archive/` 的 `task.json`，按 `createdAt` / `completedAt` 归类 |
| session-recording | 读取 `workspace/{dev}/journal-*.md`，抽取 `^## Session N: ...` 标题 |
| cli-commands | `weekly-report` 是 Python `task.py` 的运行时子命令，与 Rust 二进制的 `init/scan/update/doctor/status` 分属两套入口 |
| kb-system | KB/spec 变更计数源自 `kb/prd/`、`kb/tech/`、`spec/` 的 git log 统计 |
