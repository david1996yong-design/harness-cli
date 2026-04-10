# 扫描更新产品知识库 (kb/prd)

## Goal

重新扫描代码库，更新 `.harness-cli/kb/prd/` 下所有模块的产品知识库文档，确保文档与当前代码状态一致。

## Requirements

遵循 `/hc:scan-kb` 命令的完整流程：

1. 确认 `kb/prd/` 目录存在，读取 `_module-template.md` 模板
2. 读取现有 `index.md` 了解当前模块索引
3. 分析项目结构：入口点、模型/Schema/类型定义、配置和依赖
4. 识别业务模块（按目录组织、命名模式、领域边界、公共 API）
5. 对每个模块：阅读关键源文件，按模板格式更新 `.harness-cli/kb/prd/<模块名>.md`
6. 检查是否有新增模块需要创建文档
7. 检查是否有已删除模块需要从索引移除
8. 更新 `index.md` 索引

重点：这是重新扫描，先读取现有文档，**更新而非覆盖**。

## Acceptance Criteria

* [ ] 所有现有模块文档已根据当前代码更新
* [ ] 新增模块（如有）已创建文档
* [ ] 已删除模块（如有）已从索引移除
* [ ] `index.md` 索引与实际文档一致
* [ ] 所有文档使用中文撰写
* [ ] 文档记录代码实际做了什么，而非理想状态

## Technical Notes

* KB 目录：`.harness-cli/kb/prd/`
* 模板：`.harness-cli/kb/prd/_module-template.md`
* 现有模块数量：8 个
* 不修改任何源代码，只更新 KB 文档
