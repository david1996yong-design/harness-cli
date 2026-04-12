# 更新 /hc:onboard 命令 - 补充 KB 系统与全部新命令

## 概述

当前 onboard 命令只覆盖了 6/15 个命令，完全没有提到 KB（产品知识库）系统。
需要更新 onboard 使其反映系统最新状态，让新成员获得完整的认知。

## 需求

### R1: 新增「挑战 4」引出 KB 系统
- [x] 在 Core Philosophy 中新增第四个挑战：AI 不了解产品做了什么
- [x] 引出 KB 系统作为解决方案
- [x] 说明 spec vs kb vs tasks 的三柱架构

### R2: 更新系统结构图
- [x] 在目录结构中加入 `kb/prd/` 目录
- [x] 清晰展示三柱知识体系

### R3: 补充所有命令介绍
- [x] 核心命令（8 个）详细讲解：start, before-dev, brainstorm, scan-kb + update-kb, update-spec, check + check-cross-layer, finish-work, record-session
- [x] 高级命令（4 个）分组简述：break-loop, parallel, create-command, integrate-skill

### R4: 新增 2 个工作流示例
- [x] 示例 6: 旧项目首次接入（scan → scan-kb → fill spec）
- [x] 示例 7: 大型特性并行开发（parallel 工作流）

### R5: Part 4 扩展为覆盖 KB 初始化
- [x] 检查 spec 状态（保留）
- [x] 新增检查 KB 状态
- [x] KB 未初始化时，详细 step-by-step 演示 scan → scan-kb 流程

## 验收标准

- [x] Part 1 包含 4 个挑战（含 KB 相关的挑战 4）
- [x] 系统结构图包含 kb/prd/ 目录
- [x] 所有 15 个命令都被提及（8 核心详述 + 4 高级简述 + 自身）
- [x] 7 个工作流示例
- [x] Part 3 包含 KB 初始化详细 step-by-step
- [x] 命令介绍：核心命令 WHY/WHAT/MATTERS 格式，高级命令分组简述
- [x] 同步到所有 5 个平台（claude, cursor, codebuddy, iflow, opencode）

## 相关模块参考

| 模块 | 说明 | 涉及 |
|------|------|------|
| [CLI 命令](./cli-commands.md) | init/update/scan 命令 | scan 初始化 KB |
| [模板系统](./template-system.md) | 嵌入模板系统 | onboard.md 模板文件 |

## 决策

- KB 系统介绍深度：**B) 详细 step-by-step**（在 onboard 中直接演示完整流程）
- 命令介绍格式：**A) 核心命令详讲 + 其余简述**

## 备注

- 目标文件：`embedded/templates/claude/commands/hc/onboard.md`
- 同步更新 cursor 等平台版本（如存在）
