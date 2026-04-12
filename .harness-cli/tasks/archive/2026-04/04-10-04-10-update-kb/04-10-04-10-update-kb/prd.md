# update-kb: 增量更新产品知识库

## Goal

在现有 `/hc:scan-kb`（全量扫描）基础上，新增 `/hc:update-kb` 命令，支持基于 git diff 的增量更新，只更新受代码变更影响的 KB 模块文档，减少不必要的全量扫描开销。

## Requirements

1. 创建 `/hc:update-kb` Claude command（一个 `.md` 文件）
2. 命令流程：
   - 确认 KB 目录和已有模块文档存在
   - 读取模块模板 `_module-template.md`
   - 读取 `index.md` 了解当前模块索引
   - 通过 `git diff --name-only HEAD~N`（默认 N=10，可由用户指定 commit range）获取变更文件列表
   - 读取每个模块文档的「关键文件」表，建立 文件→模块 映射
   - 过滤出受影响的模块列表
   - 对每个受影响的模块：读取现有文档 + 变更的源文件，增量更新文档内容
   - 检测是否有变更文件不属于任何现有模块 → 提示创建新模块文档
   - 检测已删除的模块（关键文件全部不存在） → 从 index.md 移除
   - 更新 index.md 的模块索引和简述
3. 文件位置：同时写入 `embedded/templates/claude/commands/hc/update-kb.md` 和 `.claude/commands/hc/update-kb.md`（内容相同）
4. 格式与风格：与现有 `scan-kb.md` 保持一致（中文、步骤式指令）

## Acceptance Criteria

* [ ] `embedded/templates/claude/commands/hc/update-kb.md` 文件存在
* [ ] `.claude/commands/hc/update-kb.md` 文件存在（内容一致）
* [ ] 命令指令包含：确认 KB 目录、读取模板、读取索引、git diff 获取变更、映射模块、增量更新、新模块检测、删除模块检测、更新索引
* [ ] 命令默认使用 `git diff --name-only HEAD~10` 作为变更检测基准
* [ ] 命令支持用户指定 commit range 或分支对比
* [ ] 只更新受影响的模块，不触碰未变更模块
* [ ] 文档使用中文撰写
* [ ] 格式与 scan-kb.md 风格一致

## Out of Scope

* 不修改 Rust CLI 代码
* 不修改现有 scan-kb.md
* 不添加其他 AI 平台（Cursor 等）的对应命令（后续由 update 命令同步）

## Technical Notes

* 参考文件：`embedded/templates/claude/commands/hc/scan-kb.md`（现有全量扫描命令）
* KB 模块模板：`.harness-cli/kb/prd/_module-template.md`
* 现有模块索引：`.harness-cli/kb/prd/index.md`
* 每个模块文档的「关键文件」表是建立映射的关键数据源
* 变更检测策略：默认 `HEAD~10`，AI agent 可根据上下文调整（如对比特定分支）
