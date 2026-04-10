# brainstorm: 三大工作模式 (dev/debug/arch)

## Goal

为 harness-cli 工作流系统增加明确的工作模式区分，让 AI 在不同场景下采用不同的流程和策略：
- **Dev 模式**（开发模式）：当前默认的开发流程，构建新功能或处理明确的开发任务
- **Debug 模式**（问题排查模式）：面对 bug/问题时的结构化排查流程，主动向用户索要关键日志
- **Arch 模式**（架构优化模式）：review tech specs 和代码，生成架构优化建议

## What I already know

### 现有系统分析
* `/hc:start` 是当前唯一入口，通过 Task Classification 分流（Question/Trivial/Simple/Complex）
* `/hc:brainstorm` 处理复杂需求的发散→收敛探索
* `/hc:break-loop` 是 bug 修复后的事后分析工具（不是 debug 流程本身）
* 现有命令支持 `ARGUMENTS` 传参（通过 `<command-args>` 标签）
* 命令模板在 `embedded/templates/claude/commands/hc/*.md` 中定义
* 命令通过 `harness-cli init --claude` 提取到 `.claude/commands/hc/`
* 任务系统支持 `implement.jsonl`, `check.jsonl`, `debug.jsonl` 三种上下文注入

### 现有相关命令
* `/hc:start` — 开发会话入口，包含任务分类和工作流路由
* `/hc:brainstorm` — 需求发现（Diverge→Converge 模式）
* `/hc:break-loop` — bug 事后分析（Root Cause → Prevention）
* `/hc:before-dev` — 开发前的规范阅读
* `/hc:check` / `/hc:check-cross-layer` — 代码质量检查

### 约束
* 命令名遵循 kebab-case
* 命名约定：`start`, `before-*`, `check-*`, `record-*`, `update-*`
* 单个命令文件是 Markdown 模板，嵌入 Rust binary
* Python 脚本处理任务管理（task.py）

## Assumptions (temporary)

* Dev 模式基本保持现有 `/hc:start` 的流程不变
* Debug 模式需要一个全新的结构化流程（不是简单调用 `/hc:break-loop`）
* Arch 模式需要自动扫描能力（读 spec、读代码、生成建议）
* 三个模式可能共享部分基础设施（任务创建、上下文注入、记录）

## Open Questions

（无剩余阻塞问题）

## Decision (ADR-lite)

### 触发方式
**Context**: 三种模式需要统一入口还是独立命令
**Decision**: 方案 A — `/hc:start` 参数路由
- `/hc:start`（默认 dev）
- `/hc:start debug`
- `/hc:start arch`
**Consequences**: 统一入口，复用初始化逻辑；`start.md` 文件会变大，需按模式分段组织

### Debug 模式流程
**Context**: Debug 模式需要多深的排查流程
**Decision**: 完整诊断流程（7 步）
1. **收集信息** — 分步索要：现象描述 → 错误日志 → 复现步骤 → 环境信息
2. **创建 debug 任务** — 记录到任务系统，追踪排查过程
3. **自动扫描** — 根据日志关键词搜索代码 + 读 git blame/log 查看最近变更
4. **形成假设** — 列出假设 + 排查优先级
5. **逐一验证** — 每个假设尝试验证（读代码/加 log/写测试复现）
6. **实施修复** — 确认根因后修复
7. **知识沉淀** — break-loop 分析 + 更新 spec
**Consequences**: 流程较重但适合复杂问题；简单 bug 用户可在步骤 1 直接给出足够信息加速流程

### Arch 模式输出形式
**Context**: 架构分析结果如何呈现、如何转化为行动
**Decision**: 方案 3 — 报告 + 可选任务拆解
- 先自动扫描生成架构分析报告
- 按优先级列出问题和建议
- 然后问用户是否要拆成具体开发任务
- Yes → 自动创建子任务；No → 到此结束
**Consequences**: 灵活度高，用户可以只用来做 review 而不产生额外工作项

## Requirements

### 模式触发
* `/hc:start` — 默认 dev 模式（保持现有行为不变）
* `/hc:start debug [问题描述]` — 进入 debug 模式
* `/hc:start arch [模块名]` — 进入 arch 模式（可选指定扫描范围）
* 参数解析在 `start.md` 初始化阶段完成，然后路由到对应模式流程

### Dev 模式（默认）
* 保持现有 `/hc:start` 的完整流程不变
* Task Classification: Question / Trivial / Simple / Complex
* 复杂任务触发 brainstorm → Task Workflow

### Debug 模式（完整诊断 7 步）
1. **收集信息** — 分步索要：现象描述 → 错误日志 → 复现步骤 → 环境信息（一次一问）
2. **创建 debug 任务** — `task.py create "debug: <简述>"` 记录到任务系统
3. **自动扫描** — 根据日志关键词搜索代码 + git blame/log 查看最近变更
4. **形成假设** — 列出 2-3 个假设 + 排查优先级，让用户确认方向
5. **逐一验证** — 读代码 / 加 log / 写测试复现
6. **实施修复** — 确认根因后修复，调用 implement agent
7. **知识沉淀** — 自动触发 break-loop 分析 + 更新 spec（如因规范缺失导致）

### Arch 模式（报告 + 可选任务拆解）
1. **确定范围** — 全项目 or 用户指定模块（`/hc:start arch <module>`）
2. **自动扫描** — 读 `.harness-cli/spec/`、`kb/`、源码结构
3. **生成报告** — 架构分析报告，按优先级列出问题和优化建议
4. **讨论确认** — 用户 review 报告
5. **可选任务拆解** — 问用户是否拆成子任务，Yes → 自动创建

### 模式切换
* Dev 模式中发现是 bug → 支持切换到 debug 模式（保留已有上下文）
* Debug 修完后自动回到 dev 模式继续
* Arch 生成的子任务执行时自动进入 dev 模式

## Acceptance Criteria

* [ ] `/hc:start` 无参数时行为与现有完全一致（dev 模式）
* [ ] `/hc:start debug` 进入 7 步诊断流程，分步收集信息
* [ ] `/hc:start debug "某某报错"` 携带初始问题描述跳过第一问
* [ ] `/hc:start arch` 全项目扫描，生成架构分析报告
* [ ] `/hc:start arch cli-commands` 指定模块扫描
* [ ] Arch 报告输出后提供任务拆解选项
* [ ] Dev 模式中可切换到 debug 模式
* [ ] Debug 修复完成后自动衔接 break-loop 知识沉淀

## Definition of Done (team quality bar)

* Tests added/updated (unit/integration where appropriate)
* Lint / typecheck / CI green
* Docs/notes updated if behavior changes
* Rollout/rollback considered if risky

## Out of Scope (explicit)

* 其他模式（review、perf 等）— 未来扩展，本次不做
* 多模式并行（同时处于 debug + arch）— 不支持
* 模式路由的 Rust 代码改动 — 本次只改 Markdown 模板，参数解析在模板内完成
* Debug 模式的自动日志采集（如自动跑命令抓 log）— 依赖用户提供

## Technical Approach

### 改动范围
仅修改 Markdown 模板文件，不涉及 Rust 代码改动：
- **修改** `embedded/templates/claude/commands/hc/start.md` — 增加模式参数解析和路由逻辑
- 新增模式流程直接写在 `start.md` 中（按段落组织：Dev Mode / Debug Mode / Arch Mode）

### Arch 模式扫描维度（5 维）
1. **代码结构** — 模块划分、职责单一、依赖方向
2. **Spec 覆盖度** — 哪些模块有 spec、哪些缺失
3. **代码重复/坏味道** — 重复逻辑、过长函数、深嵌套
4. **一致性** — 错误处理风格、命名约定、API 设计一致性
5. **可扩展性** — 新增功能改动范围、扩展点设计

### 模式路由逻辑（在 start.md 中）
```
ARGUMENTS 解析:
  空 / 非 debug|arch → Dev 模式（现有逻辑）
  "debug ..." → Debug 模式
  "arch ..."  → Arch 模式
```

## Technical Notes

* 命令模板路径：`embedded/templates/claude/commands/hc/`
* 现有命令数量：16 个
* `/hc:start` 已有 ARGUMENTS 支持机制
* task.py 已支持 `debug.jsonl` 上下文类型（为 debug agent 准备）

## 相关模块参考

| 模块 | 说明 | 关键文件 |
|------|------|----------|
| [模板系统](./template-system.md) | 编译时嵌入模板 + 远程模板获取 + 哈希追踪 | `src/templates/`、`embedded/` |
| [平台配置器](./configurators.md) | 按平台提取嵌入模板到用户项目 | `src/configurators/` |
