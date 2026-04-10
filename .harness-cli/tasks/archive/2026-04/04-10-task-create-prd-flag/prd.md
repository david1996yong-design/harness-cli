# 给task create增加--prd参数

## 概述

task.py create 支持 --prd 参数，传入已有PRD文件路径时直接使用，未传入时生成模板并触发脑爆

## 需求

- [ ] （请填写具体需求）

## 验收标准

- [ ] （请填写验收条件）

## 相关模块参考

以下为项目已有的业务模块（来自 `kb/prd/index.md`），请标注本任务涉及的模块：

| 模块 | 说明 | 关键文件 |
|------|------|----------|
| [CLI 命令](./cli-commands.md) | 三个顶层子命令（init/update/scan）的完整生命周期 | `src/commands/` |
| [AI 工具注册表](./ai-tool-registry.md) | 13 个 AI 工具的中心配置注册表 | `src/types/ai_tools.rs` |
| [平台配置器](./configurators.md) | 按平台提取嵌入模板到用户项目 | `src/configurators/` |
| [模板系统](./template-system.md) | 编译时嵌入模板 + 远程模板获取 + 哈希追踪 | `src/templates/`、`embedded/` |
| [迁移系统](./migration-system.md) | 版本间安全文件迁移（重命名/删除） | `src/migrations/`、`embedded/manifests/` |
| [项目检测](./project-detection.md) | 自动识别项目类型和 monorepo 结构 | `src/utils/project_detector.rs` |
| [文件管理](./file-management.md) | 冲突感知文件写入 + 版本比较 + 代理支持 | `src/utils/file_writer.rs`、`src/utils/compare_versions.rs` |
| [常量](./constants.md) | 路径定义和版本常量 | `src/constants/` |

## 备注

（补充说明、技术方案、参考链接等）
