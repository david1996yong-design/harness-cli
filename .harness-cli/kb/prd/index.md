# 产品知识库（KB/PRD）

> 产品做什么 -- 业务逻辑、功能规则、领域模型

## 三层知识体系

| 目录 | 定位 |
|------|------|
| `spec/` | 如何写代码（规范、模式、指南） |
| `kb/prd/` | 产品做什么（业务逻辑、功能、规则） |
| `kb/tech/` | 系统怎么搭的（架构、组件、决策） |
| `tasks/` | 接下来做什么（当前工作项） |

## 使用方式

- AI agent 在开发前读取本目录，获取业务上下文
- 全量扫描由 `/hc:scan-kb` 触发（AI 命令，按模块模板重建所有文档）
- 增量更新由 `/hc:update-kb` 触发（基于 git diff 只更新受影响的模块）

## 文档索引

| 文档 | 简述 |
|------|------|
| `_module-template.md` | 产品知识库模块文档模板（模块概述/关键文件/核心功能/数据流/业务规则/关系） |
| `cli-commands.md` | CLI 命令系统：init/scan/update 三个核心子命令的职责、参数与流程 |
| `ai-tool-registry.md` | AI 工具注册表：13 个 AI 编程平台的静态配置元数据源 |
| `platform-configurators.md` | 平台配置器：为各 AI 平台复制嵌入式模板并构造托管目录集合 |
| `template-system.md` | 嵌入式模板系统：使用 rust-embed 编译时嵌入 14 个模板目录 |
| `migration-system.md` | 版本迁移系统：跨版本升级时处理文件重命名、删除和哈希保护 |
| `file-management.md` | 文件管理：带冲突处理（Ask/Force/Skip/Append）的文件写入 |
| `project-detection.md` | 项目检测：自动识别项目类型（Frontend/Backend/Fullstack）和 monorepo 结构 |
| `version-management.md` | 版本管理：完整 semver 比较、版本常量和路径常量体系 |
| `template-fetcher.md` | 远程模板获取：从 GitHub/GitLab/自托管 registry 下载模板并追踪哈希 |
| `kb-system.md` | 知识库系统：创建 kb/prd 和 kb/tech 目录骨架，承载 AI 命令接口 |

<!-- 以上由 scan-kb 自动生成 -->
