# brainstorm: 改进 PRD 模板以支持历史迭代引用

## Goal

改进 harness-cli 自动生成的 PRD 模板（`_generate_prd` in `task_store.py`），让每个任务的 PRD 能够**引用和记录历史迭代**，解决"同一个功能反复做 v1 / v2 / v3，但没有任何跨任务的追溯链路"的痛点。

## What I already know

### 当前 PRD 模板（代码位置：`.harness-cli/scripts/common/task_store.py:57-122`）

自动生成的结构只有：

```
# {title}
## 概述
## 需求
## 验收标准
## 相关模块参考   ← 自动从 kb/prd/index.md 抓表格
## 备注
```

**完全没有**指向前置任务、历史版本、关联决策的字段。

### 真实痛点案例（就在本仓库）

- `tasks/archive/2026-04/04-10-kb-tech-layer/prd.md` — v1
- `tasks/04-10-kb-tech-layer-v2/prd.md` — v2（已 completed）

两份 PRD 内容 ~90% 重复，但 v2 **完全没有**提到：
- 为何要做 v2（v1 哪里不够 / 被撤回 / 方向调整）
- 相对 v1 的差异点
- v1 哪些决策继承、哪些作废

未来读 v2 的人（或 AI agent）无法理解"这事怎么来的"。

### task.json 现有关系字段

- `parent` / `children` / `subtasks` — 仅用于父子分解
- **没有** `supersedes`、`supersededBy`、`previousVersion`、`relatedTasks`、`iteration` 等迭代性字段

### 已有的相关基础设施

- `kb/prd/<module>/` — 模块级的长期产品知识（跨任务沉淀）
- `kb/tech/decisions.md` — 架构决策 ADR-lite（跨任务的技术决策）
- `scripts/task.py archive` — 任务归档到 `tasks/archive/YYYY-MM/`
- `/hc:scan-kb` / `/hc:update-kb` — 从已完成任务回流 KB

这些解决的是"模块知识长期化"，但**per-task 的迭代史没有归宿**。

## Assumptions (temporary)

1. 用户想要的"历史迭代引用"主要是**同一功能的多次迭代之间的追溯链**（v1 → v2 → v3），而不是"模块长期知识"。
2. 引用应该是**双向**的：v2 能找到 v1，v1 能知道被 v2 取代。
3. 引用的载体应该是 task.json 的结构化字段（用于脚本查询）+ PRD 模板的文字区块（用于人和 AI 阅读）。
4. "历史迭代"不仅包括前置任务，可能也包括 PRD 自身在本任务中的决策变更记录（本次迭代内的决策史）。

## Scope Decision (2026-04-11)

用户确认范围为 **A + B + C 三个维度全覆盖**：

- **A — 跨任务版本链**：v1 → v2 → v3 双向追溯（supersedes / supersededBy）
- **B — 关联任务网络**：extends / fixes / depends-on / inspired-by 等引用关系
- **C — 单任务决策日志**：本次 brainstorm/实现过程中的决策演变记录

MVP 建议从 A 切入（最痛），但方案需保留 B/C 的扩展点。

## Expansion Sweep

### Future evolution
- 可视化迭代图谱（v1→v2→v3 链、任务之间的 extends/fixes 关系网）
- `kb/prd/<module>/history.md` 自动聚合所有 supersedes / relatedTasks，形成模块演进史
- AI agent 利用 decision log "不重复犯错"（读前置任务的 lessons）

### Related scenarios
- `/hc:archive`：归档 v1 时应在 v1 的 `supersededBy` 写入 v2
- `/hc:update-kb`：把 decision log 回流到 `kb/prd/<module>/` 或 `kb/tech/decisions.md`
- `task.py list / status`：显示 v 链（v1 → v2 → v3）
- `/hc:start` / `/hc:brainstorm`：新任务启动时，如有前置版本自动拉入其 decisions / out-of-scope

### Edge cases
- 用 dir name 作引用 key；任务 rename / move 会断链 → 需约定"已发布任务不改名"
- 循环引用（A.supersedes=B, B.supersedes=A）→ 创建时校验
- v1 归档到 `archive/2026-04/` 后，v2 的反向查找仍要能找到（dir 查找要覆盖 archive 子树）
- v2 被取消 → 需要清掉 v1 的 `supersededBy`
- 老的已归档任务没有新字段 → 读取代码要兼容缺字段

## Decision (ADR-lite, 2026-04-11)

**Context**：需要让 PRD 承载 A+B+C 三维度的迭代史（版本链 / 关联任务 / 决策日志），同时避免过度工程化。

**Decision**：采用 **Approach A — 纯 PRD 模板扩展**，不动 task.json schema。

**Consequences**：
- ✅ 改动面最小，只改 `_generate_prd` 和可能的 `task.py create` 一个 flag
- ✅ 零迁移、老任务零影响
- ✅ 所有信息在单一 PRD 文件内，AI agent 读一份就够
- ⚠️ 双向链需要人工（或命令辅助）同步维护——v2 建立后要手动去更新 v1 的 PRD
- ⚠️ 无法脚本查询 v 链，`task.py list` 暂时看不到迭代关系
- 🔓 保留将来升级为 B 的扩展点：section 标题和格式向结构化字段对齐，后续若需升级，解析现有 markdown 即可迁移到 task.json

## Decision Refinement (2026-04-11)

在 Approach A 内进一步选定 **A1 — 纯模板，零自动化**：

- **不加** `--supersedes` flag
- **不动** `task.py create` 命令签名
- **只改** `_generate_prd` 一个函数，往模板加 3 个 section
- v1/v2 之间的引用路径完全人工敲（或由 AI agent 在 brainstorm 时协助填入）

## Open Questions

- [P] `/hc:brainstorm` 命令模板是否同步更新？（见下一个问题）

## Requirements (evolving)

- （待方案选定后收敛）

## Acceptance Criteria (evolving)

- [ ] （待收敛）

## Definition of Done

- PRD 模板改动落到 `task_store.py:_generate_prd`
- task.json schema 如有新字段，同步更新到 create/list/archive 等涉及读写的代码路径
- 更新 `/hc:brainstorm` 等模板使其产出符合新 schema 的 PRD
- 有 1-2 个真实任务用新模板回填验证可用性
- 不破坏已有归档任务的 PRD 读取

## Out of Scope (explicit)

- （待收敛）

## Technical Notes

- 模板生成：`.harness-cli/scripts/common/task_store.py:57-122` (`_generate_prd`)
- task schema：`.harness-cli/scripts/common/task_store.py:207-253` (`cmd_create` 构造 task_data)
- 真实 v1/v2 案例：`tasks/archive/2026-04/04-10-kb-tech-layer/` 和 `tasks/04-10-kb-tech-layer-v2/`
- 已有 ADR-lite 概念在 `kb/tech/decisions.md`，与本任务可能有交集

## Research Notes

### 类似工具的做法

- **Linear**：每个 issue 可设 `parent` / `blocks` / `related to` / `duplicate of`；没有专门的 "supersedes"，但 "duplicate of" 承担了类似角色
- **Jira**：Link 类型开放（relates to / blocks / is blocked by / clones / is cloned by / duplicates / is duplicated by），关系双向自动维护
- **GitHub Issues / Linked PRs**：`Closes #123` / `See also #456` 是文字约定，机器从评论里 parse
- **Architecture Decision Records (ADR)**：每个决定独立文件，用 `Status: Superseded by [ADR-0042]` 链接上下游
- **RFC 流程（Rust / Python）**：RFC 文档开头有 `Related:` / `Supersedes:` / `Superseded by:` 字段
- **Notion / Obsidian**：靠 backlinks，双向链不需要手工维护

### 收到的启示

1. **双向链最好自动维护**（Jira / Notion 的经验）→ CLI 命令写一端时同步写另一端
2. **关系类型不宜太多**（Linear 的精简派胜过 Jira 的冗杂派）→ 先只做 `supersedes / supersededBy / related`
3. **结构化 + 叙事并存**（ADR 模式）→ task.json 存关系，PRD 存原因
4. **引用 key 要稳定**（GitHub 用全局自增 ID）→ 我们用 dir name，约定不改名

### 本项目约束

- task.json 已有 schema，新增字段不能破坏老任务
- `_generate_prd` 只在 create 时跑一次，不重渲染
- 归档后 dir 路径变动，但 dir name 保持（`find_task_by_name` 已支持跨 active/archive 查找）
- PRD 是人和 AI 共同阅读的"真相源"，不能把关键信息藏到 task.json 让 PRD 看不见

## Approaches（2–3 候选方案）

### Approach A — 最小侵入：纯 PRD 模板扩展（无 schema 变更）

**How it works**
- `_generate_prd` 生成的 PRD 新增 3 个 section（初始为空/提示文字）：
  ```
  ## 历史版本（A）
  - 前置版本：<none>
  - 本版变更：<none>

  ## 相关任务（B）
  - <none>

  ## 决策日志（C）
  - 2026-04-11 — 初始方案：<待定>
  ```
- `task.py create` 新增 `--supersedes <dir-name>` flag：有值时，种入 PRD 第一行链接（但不写 task.json）
- 完全不动 task.json schema

**Pros**
- 改动量最小（只改 `_generate_prd` + create 命令新增一个 flag）
- 零迁移负担，老任务完全不受影响
- 模板即文档，人和 AI 读 PRD 就看得到

**Cons**
- 没有结构化关系，`task.py list` 无法显示 v 链
- 双向链要靠人工维护（v2 建好后要手动去改 v1 的 PRD）
- 无法自动聚合到 `kb/` 或生成图谱
- 不符合"SSOT"原则：同样的事实可能在多个 PRD 里漂移

---

### Approach B — 推荐：结构化关系 + 叙事日志分层（task.json + PRD 双写）

**How it works**

**1. task.json 扩展 schema**（新增 3 个字段，缺失即视为 null/空）：
```jsonc
{
  "supersedes": "04-10-kb-tech-layer",        // A：前置版本（单值）
  "supersededBy": "04-10-kb-tech-layer-v2",   // A：被谁取代（双向自动维护）
  "relatedTasks": [                            // B：关联任务网络
    {"task": "04-10-scan-kb", "relation": "extends", "note": "复用 scan 框架"},
    {"task": "04-09-old-kb-layout", "relation": "fixes", "note": "解决目录结构偏差"}
  ]
}
```
关系枚举（精简派）：`extends | fixes | depends-on | inspired-by`

**2. PRD 模板扩展**（`_generate_prd` 生成时从 task.json 渲染 A/B，C 留空模板）：
```markdown
## 历史版本
（create 时若 --supersedes 已给，渲染链接到 v1 prd 文件；否则写 "无"）
- v1：[04-10-kb-tech-layer](../../archive/2026-04/04-10-kb-tech-layer/prd.md)
  - 结论：<v1 的一句话总结>
  - 本版差异：<待填>

## 相关任务
（从 task.json relatedTasks 渲染，create 时为空）

## 决策日志（C）
- YYYY-MM-DD — <决策> — <原因>
```

**3. 新命令（保证双向链）**：
```bash
task.py supersede <new-task> --of <old-task>
  # 自动：new.supersedes=old, old.supersededBy=new，两边 PRD 都追加互相的链接
task.py relate <task-a> <task-b> --as extends|fixes|depends-on|inspired-by --note "..."
  # 双向写入 relatedTasks
```

**4. 命令联动**：
- `/hc:brainstorm`：新任务由 `--supersedes` 启动时，自动读 v1 PRD 的 `Decision` / `Out of Scope` 回填到新 PRD 的"What I already know"
- `/hc:archive`：归档不破坏引用（dir name 稳定）
- `task.py list / status`：输出列里显示 `→ supersededBy` 标记（可选）

**5. C（决策日志）保持纯 markdown**：
- 不进 task.json（因为是叙事性），就在 PRD 里维护
- `/hc:brainstorm` 在 Q&A Loop 每答一个问题时，追加一行到 `## 决策日志`

**Pros**
- SSOT：关系存 task.json，叙事存 PRD，各司其职
- 可脚本查询，可生成 v 链图 / 任务图谱
- 双向链由命令自动维护，人不会忘
- C 保留叙事灵活性，不被强 schema 绑架
- 向后兼容：老任务缺字段视为 null，不报错

**Cons**
- 改动面较大：schema + create + 新增 2 个命令 + 模板 + brainstorm 流程
- 需要写校验（禁止循环、禁止自引用、dir name 解析要覆盖 archive）
- PRD 与 task.json 可能漂移（用户手改 PRD 但没同步 json）→ 需约定"关系改动走命令、不手改"

---

### Approach C — 分离文件：独立 `history.md`（PRD 保持极简）

**How it works**
- PRD 不加新 section，保持现状
- 每个任务目录下新增可选文件 `history.md`（有迭代时才创建，由 `task.py supersede` 或 `task.py relate` 自动生成）
- `history.md` 承载 A+B+C 全部内容
- task.json 依然加 `supersedes/supersededBy/relatedTasks`（否则没法自动维护）

**Pros**
- PRD 保持简洁，老读者习惯不变
- "历史"是第一等公民，不和需求混杂
- AI agent 明确知道"要看迭代史 → 读 history.md"

**Cons**
- 两个文件读起来分裂，AI agent 经常漏读 history.md
- 新增一个文件类型，`/hc:start` / `/hc:check` / `/hc:update-kb` 全都要更新去读它
- 绝大多数任务不会有 history，会产生"空文件或不存在？"的判断负担

---

## 快速对比

| 维度 | A（纯 PRD）| B（推荐，双写）| C（独立文件）|
|------|------------|----------------|--------------|
| 改动面 | 小 | 中 | 中 |
| SSOT | ❌ | ✅ | ✅ |
| 双向链自动维护 | ❌ | ✅ | ✅ |
| 可脚本查询 | ❌ | ✅ | ✅ |
| AI 读取路径 | 单文件 | 单文件 | 双文件 |
| 迁移负担 | 无 | 低（新字段兼容 null）| 低 |
| 长期可扩展 | 差 | 好 | 好 |

