# 知识库系统

> 管理产品知识库（kb/prd）和技术架构知识库（kb/tech）的目录骨架创建，并承载 AI 命令用于填充内容

## 模块概述

知识库系统是 harness-cli 三层知识体系（spec / kb-prd / kb-tech）中的知识库部分。CLI 的 `scan` 子命令负责创建 `kb/prd/` 和 `kb/tech/` 两个目录骨架（各自包含 `index.md` 和 `_module-template.md`），实际的文档内容由 AI 命令生成。知识库分为两个层次：产品知识库（`kb/prd/`）按模块组织，记录「产品做什么」；技术知识库（`kb/tech/`）使用 5 个固定文档（overview、component-map、data-models、decisions、cross-cutting），记录「系统怎么搭的」。

## 关键文件

| 文件路径 | 职责 |
|----------|------|
| `src/commands/scan.rs` | scan 命令实现：`ScanOptions { force }`、`scan`、`create_kb_prd`、`create_kb_tech`；在 `.harness-cli/` 不存在时提示先运行 `init` |
| `src/templates/markdown.rs` | 通过 `md_template!` 宏暴露 `kb_prd_index_content()`、`kb_prd_module_template_content()`、`kb_tech_index_content()`、`kb_tech_module_template_content()` |
| `embedded/templates/markdown/kb/prd/index.md.txt` | 产品知识库索引模板（含三层体系说明和文档索引表） |
| `embedded/templates/markdown/kb/prd/module-template.md.txt` | 产品知识库模块文档模板（模块概述/关键文件/核心功能/数据流/业务规则/关系） |
| `embedded/templates/markdown/kb/tech/index.md.txt` | 技术知识库索引模板（列出 5 个固定文档） |
| `embedded/templates/markdown/kb/tech/module-template.md.txt` | 技术知识库文档模板与写作指引 |
| `embedded/templates/claude/commands/hc/update-kb.md` | AI 命令：基于 git diff 增量更新 `kb/prd/` |
| `embedded/templates/claude/commands/hc/scan-kb-tech.md` | AI 命令：全量扫描生成 `kb/tech/` 下的 5 个固定文档 |
| `src/constants/paths.rs` | `constructed::KB`、`KB_PRD`、`KB_TECH` 路径常量；`dir_names::KB`、`KB_PRD`、`KB_TECH` 原子目录名 |

## 核心功能

### KB 目录骨架创建

- **业务规则**: `scan` 命令为 `kb/prd/` 和 `kb/tech/` 创建目录，并写入 `index.md` 和 `_module-template.md` 两个文件
- **触发条件**: 用户运行 `harness-cli scan`，且项目已通过 `init` 创建 `.harness-cli/` 目录
- **处理流程**:
  1. 检查 `.harness-cli/` 是否存在；不存在时打印红色错误 `Error: .harness-cli/ not found. Run 'harness-cli init' first.` 并返回
  2. 若 `options.force == true`，通过 `set_write_mode(WriteMode::Force)` 设置全局写入模式为强制覆盖
  3. 调用 `create_kb_prd(cwd)`：`ensure_dir` 创建 `.harness-cli/kb/prd/` -> `write_file` 写入 `index.md` 和 `_module-template.md`
  4. 调用 `create_kb_tech(cwd)`：`ensure_dir` 创建 `.harness-cli/kb/tech/` -> `write_file` 写入 `index.md` 和 `_module-template.md`
  5. 最后打印绿色成功消息以及下一步提示：运行 `/hc:scan-kb` 和 `/hc:scan-kb-tech`

### 三层知识体系

- **spec/**: 如何写代码（规范、模式、指南）
- **kb/prd/**: 产品做什么（业务逻辑、功能、规则）—— 按模块组织，每个模块一个 `.md` 文件
- **kb/tech/**: 系统怎么搭的（架构、组件、决策）—— 5 个固定文档
- **tasks/**: 接下来做什么（当前工作项）

### 产品知识库结构

- **`index.md`**: 三层体系说明 + 文档索引表（模块文件 + 简述），顶部注释 `<!-- 以下由 scan-kb 自动生成 -->` 标记 AI 生成区域
- **`_module-template.md`**: 单个模块文档的标准结构（模块概述 / 关键文件表 / 核心功能 / 数据流 / 业务规则 / 与其他模块的关系）
- **每个业务模块**: 一个独立的 `<module-name>.md` 文件，遵循 `_module-template.md` 格式

### 技术知识库结构

- **`index.md`**: 三层体系说明 + 定位说明 + 固定的 5 个文档索引
- **`_module-template.md`**: 定义 5 个固定文档的结构要求和写作指引
- **`overview.md`**: 系统全景 —— 技术栈、核心组件、系统边界
- **`component-map.md`**: 组件关系 —— 依赖关系图、调用链、数据流、依赖方向原则
- **`data-models.md`**: 核心数据结构与 Schema
- **`decisions.md`**: 架构决策记录（ADR-lite 格式）
- **`cross-cutting.md`**: 横切关注点 —— 错误处理、日志、配置、共享工具、中间件

### AI 命令接口

- **`scan-kb`（AI 命令，文档引用但无独立命令文件）**:
  - 产品知识库的全量扫描由 `/hc:scan-kb` 触发，但 `embedded/templates/claude/commands/hc/` 目录中**没有独立的 `scan-kb.md` 命令文件**
  - 当前 CLI 代码（`scan.rs`）和 markdown 模板（`kb/prd/index.md.txt`）仍然引用该命令名
  - 实际工作流程由 AI 按照 `update-kb.md` 中描述的步骤手动执行全量扫描，或沿用 `scan-kb-tech.md` 的模式自行分析代码
- **`scan-kb-tech`（AI 命令，全量扫描 `kb/tech/`）**:
  - 对应文件 `embedded/templates/claude/commands/hc/scan-kb-tech.md`
  - 分析项目技术栈和组件边界 -> 按模板生成 5 个固定文档 -> 更新 `index.md`
- **`update-kb`（AI 命令，增量更新 `kb/prd/`）**:
  - 对应文件 `embedded/templates/claude/commands/hc/update-kb.md`
  - 执行步骤：读取 `_module-template.md` 和 `index.md` -> `git diff --name-only HEAD~10`（或用户指定范围）获取 `CHANGED_FILES` -> 解析每个模块文档的「关键文件」表建立文件-模块映射 -> 过滤受影响模块 -> 逐个增量更新 -> 识别未映射文件并提示创建新模块 -> 检测所有关键文件已删除的模块并提示移除 -> 更新 `index.md`

## 数据流

```
harness-cli scan (CLI)
  -> 检查 .harness-cli/ 存在
  -> ensure_dir kb/prd/ + write_file index.md, _module-template.md
  -> ensure_dir kb/tech/ + write_file index.md, _module-template.md
  -> 提示下一步运行 AI 命令

/hc:scan-kb (AI，无独立命令文件)
  -> AI 全量扫描代码 -> 按模板生成 kb/prd/<module>.md

/hc:update-kb (AI)
  -> 读取模板和索引 -> git diff 获取变更文件
  -> 建立文件-模块映射 -> 过滤受影响模块
  -> 增量更新模块文档 -> 检测新增/删除模块 -> 更新索引

/hc:scan-kb-tech (AI)
  -> 读取模板 -> 分析技术栈、组件、数据结构
  -> 生成 overview/component-map/data-models/decisions/cross-cutting
  -> 更新 kb/tech/index.md
```

## 业务规则

- `scan` 命令的前置条件：`.harness-cli/` 目录必须已存在（通常由 `init` 创建），否则直接退出并提示
- `scan --force` 将全局 `WriteMode` 设置为 `Force`，后续所有 `write_file` 调用会直接覆盖已存在的文件
- 未加 `--force` 时，`scan` 会复用默认的 `Ask` 写入模式，由 `file_writer::write_file` 处理冲突
- `kb/prd/` 采用「按模块组织」的结构，每个业务模块一个独立 `.md` 文件；模块数量和命名由 AI 扫描决定
- `kb/tech/` 采用固定的 5 个文档结构（overview、component-map、data-models、decisions、cross-cutting），不随项目变化
- 所有知识库文档使用中文撰写
- 文档记录代码实际做了什么，而非理想状态；不记录不存在的功能或计划中的特性
- `kb/prd/index.md` 的 `<!-- 以下由 scan-kb 自动生成 -->` 注释下方的文档索引由 AI 维护
- `kb/tech/index.md` 的固定 5 个文档在模板中硬编码，AI 扫描时只需填充简述而不增减条目
- 增量更新 `kb/prd/` 时只修改与变更相关的内容，保留未受影响的部分（避免全量重写）
- 增量更新只处理源码文件变更，跳过 `.md`、`.json`、`.gitignore` 等非源码变更

## 与其他模块的关系

| 模块 | 关系 |
|------|------|
| cli-commands | `scan` 命令是 KB 目录骨架创建的入口，由 CLI 子命令分发器调用 |
| template-system | 通过 `markdown::kb_prd_index_content()` 等函数从 `embedded/templates/markdown/kb/` 读取模板内容 |
| file-management | 通过 `ensure_dir` 创建目录，通过 `write_file` 写入模板文件并处理已存在文件冲突 |
| version-management | 使用 `constructed::KB_PRD` 和 `constructed::KB_TECH` 路径常量；`dir_names::KB`、`KB_PRD`、`KB_TECH` 也在 `paths.rs` 中定义 |
| platform-configurators | `kb/prd/` 和 `kb/tech/` 属于 `all_managed_dirs()` 管理的目录范围（用于 update 命令的文件分类） |
