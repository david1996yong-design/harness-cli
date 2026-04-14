# Weekly personal report

## 概述

每周自动聚合本地数据（归档任务 / commits / journal / KB 变更），生成一份"给自己看"的周报 Markdown。v1 只做个人版，不做团队聚合、不做外部推送。事实与 AI 洞察解耦：脚本只产出确定性事实，AI 总结通过独立 slash command 追加。

## 需求

- [ ] 新增 `python3 task.py weekly-report [--week YYYY-Www] [--dev <name>]`，默认当前开发者与当前 ISO 周（周一~周日）
- [ ] 输出到 `.harness-cli/workspace/{dev}/reports/{YYYY}-W{NN}.md`；同一周重复运行幂等覆盖，不产生副本
- [ ] 报告按序包含 5 段事实区：元信息、本周任务（分组：已归档 / 进行中 / 新建未开始，字段含 slug/title/priority/kb_status/branch）、commits 摘要（按天聚合 hash + 首行）、journal 标题清单、KB/spec 文件变更条数（不展开内容）
- [ ] 报告末尾保留 `<!-- AI 总结：运行 /hc:weekly-review 生成 -->` 锚点；脚本本身不调用 AI
- [ ] session-start hook 在周日 / 周一首次启动且当周报告不存在时，在 context 追加一句友好提示；其他日子不打扰
- [ ] 新增 `/hc:weekly-review` slash command（先只做 claude 平台），读取最近一份报告，在 AI 锚点下方追加"亮点 / 阻塞 / 下周建议"三节，不覆盖事实区

## 验收标准

- [ ] 有归档任务 + 若干 commits 的仓库里，命令在 2 秒内生成 ≤ 一屏 Markdown
- [ ] 本周零任务零 commit 时，给出友好的"本周安静"报告而非报错
- [ ] 跨月周（如 03-30 ~ 04-05）能同时列出两个月的 archive
- [ ] 开发者名含空格 / 中文不报错；路径正确
- [ ] 幂等性：同一周连续运行两次，文件覆盖而非追加，也不产生 `-1` / `-2` 副本
- [ ] session-start 提示只在周日 / 周一首次会话出现，周三再开 session 不提醒
- [ ] `/hc:weekly-review` 只在 `AI 总结` 锚点下方写入，事实区字节不变

## 备注

**刻意排除**（避免功能膨胀）：邮件 / Slack / webhook 推送；代码行数、工时、效率打分；团队聚合；HTML Dashboard；跨项目聚合。这些若后续有真实需求再开新任务。

**为什么是 Python 而非 Rust 命令**：周报属于运行时聚合（读 tasks / git / journal），与 `task.py finish`、`add_session.py` 同层。Rust 二进制只管初始化与升级，不该承担运行时聚合。

**数据源**：`tasks/` + `tasks/archive/YYYY-MM/`（task.json）、`workspace/{dev}/journal-*.md`（正则抽 `^## ` 标题）、`git log --since --until --author=<dev>`、`kb/`、`spec/` 的 git diff stat。
