# 系统架构知识库（KB/Tech）

> 系统怎么搭的 -- 架构、组件关系、数据模型、技术决策

## 三层知识体系

| 目录 | 定位 |
|------|------|
| `spec/` | 如何写代码（规范、模式、指南） |
| `kb/prd/` | 产品做什么（业务逻辑、功能、规则） |
| `kb/tech/` | 系统怎么搭的（架构、组件、决策） |
| `tasks/` | 接下来做什么（当前工作项） |

## 定位说明

- **spec/** 关注「怎么写」：编码规范、命名约定、代码模式
- **kb/prd/** 关注「做什么」：业务逻辑、功能规则、领域模型
- **kb/tech/** 关注「怎么搭」：架构全景、组件关系、数据模型、技术选型与决策

## 使用方式

- AI agent 在开发前读取本目录，获取系统架构上下文
- 由 `/hc:scan-kb-tech` 全量生成

## 文档索引

| 文档 | 简述 |
|------|------|
| `_module-template.md` | Tech 文档模板与写作指引 |
| `overview.md` | Rust binary + Python scripts 混合架构，14 平台嵌入模板；17+ 核心组件，覆盖 init/scan/update 三大命令与多 agent 运行时 |
| `component-map.md` | 4 层分层架构（AI Agents / Python scripts / Common library / Rust binary / Embedded resources），3 条关键数据流（初始化 / 任务生命周期 / hook 注入） |
| `data-models.md` | 14 个核心数据结构：AITool 枚举、AIToolConfig、CommandTemplate 等 Rust 类型，以及 TaskData、TaskInfo、AgentRecord、next_action 等 Python schema |
| `decisions.md` | 13 条架构决策：rust-embed、src/templates 双层、Rust+Python 混合、平台独立、git worktree、零依赖 Python、SHA256 追踪等 |
| `cross-cutting.md` | 错误处理（anyhow + 静默降级）、日志（彩色前缀 + stream-json）、5 层配置优先级、Rust/Python 共享工具清单、Claude Code 4 类 hooks |

<!-- 以下由 scan-kb-tech 自动生成 -->

## 扫描摘要（2026-04-11）

- **技术栈**: Rust 2021 (clap/rust-embed/anyhow) + Python 3 标准库 (multi_agent / hooks / common)
- **构建**: cargo build --release (opt-level=z, lto=true, strip=true)
- **平台覆盖**: 13 个 AI CLI 工具（claude / cursor / opencode / iflow / codex / kilo / kiro / gemini / antigravity / windsurf / qoder / codebuddy / copilot）
- **入口**: `src/main.rs:11-13`（bin = harness-cli）
- **三大命令**: init / scan / update
- **13 条架构决策**，包含 2 个已知 bug（worktree task.json 同步、非 TTY init 需 --yes）

