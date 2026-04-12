# 优化 PRD 模板 — 对齐业界最佳实践

## Goal

当前自动生成的 PRD 模板只有 5 个 section（概述/需求/验收标准/相关模块参考/备注），过于简陋。需要重新设计为 7 个核心 section 的模板，使其足够引导用户和 AI 生成高质量需求文档，同时不啰嗦。高级 section（Research Notes、ADR-lite、Assumptions 等）由 brainstorm 流程渐进添加。

## Decision

**策略**: 方案 C — 核心模板 + brainstorm 扩展
**Context**: 自动生成模板（5 section）与 brainstorm 产出（12+ section）落差大。业界（Google/Stripe/Lenny）的模板都包含 Goal、Out of Scope、DoD 等 section。
**Decision**: 自动生成给 7 个核心 section，brainstorm 流程再按需添加高级 section。
**Consequences**: 简单任务不会被冗长模板拖累；复杂任务通过 brainstorm 获得完整结构。

## Requirements

### R1: 重写 `_generate_prd()` 函数

替换现有 5-section 模板为 7-section 核心模板：

```markdown
# {title}

## Goal

{description}（说明为什么做 + 做什么）

## Requirements

- [ ] （请填写具体需求）

## Acceptance Criteria

- [ ] （请填写验收条件）

## Out of Scope

（明确列出本任务不做的事情，防止范围蔓延）

## Definition of Done

- [ ] 测试已添加或更新
- [ ] Lint / 类型检查通过
- [ ] 如行为变更，文档已更新

## Technical Notes

（相关文件路径、技术约束、参考链接）

## 相关模块参考

{auto-pulled from kb/prd/index.md — 保留现有逻辑}
```

### R2: 同步更新 brainstorm.md 的 PRD 目标结构

brainstorm 的种子模板（Step 0）和最终目标结构（Step 8）应以核心模板为基础，在其上追加高级 section：

**brainstorm 追加的高级 section（按需）**:
- What I already know
- Assumptions (temporary)
- Open Questions
- Research Notes
- Decision (ADR-lite)
- Technical Approach（方案对比后的选择）

### R3: section 语言规范

- Section 名使用英文（Goal / Requirements / Acceptance Criteria / Out of Scope / Definition of Done / Technical Notes）
- 占位提示文本使用中文
- 保留"相关模块参考"中文名（因为 KB 内容是中文的）

## Acceptance Criteria

- [ ] `task.py create` 生成的 prd.md 包含 7 个核心 section
- [ ] Goal section 自动填入 description 参数
- [ ] Definition of Done 预填默认质量标准
- [ ] 相关模块参考的自动拉取逻辑不变
- [ ] brainstorm.md 的种子模板和最终目标结构与核心模板一致
- [ ] 现有任务的 prd.md 不受影响（只改生成逻辑，不改已有文件）

## Out of Scope

- 迭代历史 section（由 `04-11-improve-prd-iteration-history` 单独处理）
- KB 模块模板（`_module-template.md`）的改动
- 自动预填能力（如从 git log 推导相关文件）— 留作后续优化
- 按复杂度分级生成不同模板（方案 B）— 不做

## Definition of Done

- [ ] 测试已添加或更新
- [ ] Lint / 类型检查通过
- [ ] 如行为变更，文档已更新
- [ ] 自动生成模板与 brainstorm 目标结构对齐

## Technical Notes

### 涉及文件
| 文件 | 改动 |
|------|------|
| `.harness-cli/scripts/common/task_store.py` 行 57-122 | 重写 `_generate_prd()` 模板内容 |
| `embedded/templates/claude/commands/hc/brainstorm.md` | 同步种子模板和最终目标结构 |

### 注意事项
- `_generate_prd()` 的 `source_prd` 参数（外部 PRD 导入）逻辑不变
- `kb_section` 自动拉取逻辑不变，只改插入位置（从"备注"前移到末尾独立 section）
- brainstorm.md 中有两处 PRD 结构定义需要同步：Step 0 种子模板（约行 62-105）和 Step 8 最终结构（约行 409-443）
