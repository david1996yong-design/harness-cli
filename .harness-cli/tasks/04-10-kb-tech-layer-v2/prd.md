# KB 新增 tech 系统架构知识库

## Goal

在 `kb/prd/`（产品做什么）基础上新增 `kb/tech/`（系统怎么构建的），包含模板定义、AI scan 命令、Rust CLI 脚手架，为 AI agent 提供系统架构层面的上下文。

## Requirements

### 1. 模板定义

在 `embedded/templates/markdown/kb/tech/` 下新增嵌入模板：

**`index.md.txt`** — tech 层索引，格式参考 `kb/prd/index.md.txt`：
- 说明 tech 层定位：记录系统**怎么搭建的**（架构、组件关系、数据模型、技术决策）
- 与 spec（如何写代码）和 prd（产品做什么）的三层对照表
- 文档索引表

**`_module-template.md.txt`** — tech 文档模板，包含以下固定文档的写作指引：

| 文档 | 内容 |
|------|------|
| `overview.md` | 系统全景：技术栈清单、核心组件一句话描述、系统边界 |
| `component-map.md` | 组件关系：谁调谁、数据怎么流、依赖方向 |
| `data-models.md` | 核心数据结构 Schema：task.json、config.yaml、registry.json 等 |
| `decisions.md` | 架构决策记录（ADR-lite）：选型、取舍、原因 |
| `cross-cutting.md` | 横切关注点：错误传播、日志管道、配置管理等 |

### 2. Rust CLI 脚手架

修改 `src/commands/scan.rs`：
- `harness-cli scan` 同时创建 `kb/prd/` 和 `kb/tech/` 目录
- `kb/tech/` 包含 `index.md` 和 `_module-template.md`

修改 `src/templates/markdown.rs`：
- 新增 `kb_tech_index_content()` 和 `kb_tech_module_template_content()` 模板访问器

修改 `src/constants/paths.rs`：
- 新增 `KB_TECH` 常量（`kb/tech`）

### 3. AI Scan 命令

新增 `/hc:scan-kb-tech` Claude command（`embedded/templates/claude/commands/hc/scan-kb-tech.md`）：

流程（参考 scan-kb）：
1. 确认 `kb/tech/` 目录存在
2. 读取模板
3. 分析项目结构——关注架构维度：
   - 技术栈识别（语言、框架、构建工具、依赖）
   - 组件边界和依赖关系
   - 核心数据结构 / Schema
   - 跨模块的共享基础设施
4. 生成 5 个固定文档（overview, component-map, data-models, decisions, cross-cutting）
5. 更新 index.md
6. 输出摘要

同时复制到 `.claude/commands/hc/scan-kb-tech.md`。

### 4. 更新现有 index.md 三层对照表

更新 `kb/prd/index.md` 模板，在概述中加入 tech 层：

```
| spec/      | 如何写代码（规范、模式、指南） |
| kb/prd/    | 产品做什么（业务逻辑、功能、规则） |
| kb/tech/   | 系统怎么搭的（架构、组件、决策） |
| tasks/     | 接下来做什么（当前工作项） |
```

## Acceptance Criteria

* [ ] `embedded/templates/markdown/kb/tech/index.md.txt` 存在
* [ ] `embedded/templates/markdown/kb/tech/module-template.md.txt` 存在
* [ ] `harness-cli scan` 创建 `kb/tech/` 目录 + index.md + _module-template.md
* [ ] `src/constants/paths.rs` 有 `KB_TECH` 常量
* [ ] `src/templates/markdown.rs` 有 tech 模板访问器
* [ ] `scan.rs` 创建 kb/tech/ 结构
* [ ] scan.rs 的现有测试仍然通过
* [ ] `/hc:scan-kb-tech` command 文件在 embedded 和 .claude 下都存在
* [ ] command 指令能生成 5 个固定文档
* [ ] 所有文档使用中文
* [ ] `cargo test` 通过

## Out of Scope

* 不做 update-kb-tech 增量更新（后续任务）
* 不修改其他 AI 平台的命令

## Technical Notes

* `scan.rs`: `src/commands/scan.rs`
* `markdown.rs`: `src/templates/markdown.rs`
* `paths.rs`: `src/constants/paths.rs`
* 参考：`embedded/templates/claude/commands/hc/scan-kb.md`
* 参考：`embedded/templates/markdown/kb/prd/index.md.txt`
* 参考：`embedded/templates/markdown/kb/prd/module-template.md.txt`
