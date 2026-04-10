# 版本管理

> 提供 semver 版本号比较和 CLI/项目版本一致性检查

## 模块概述

版本管理模块负责两个核心任务：完整 semver 版本号比较（支持带数字后缀和字符串标签的预发布版本），以及在启动时检测 CLI 版本与项目版本是否一致。版本号通过 `env!("CARGO_PKG_VERSION")` 在编译时嵌入。路径常量体系（`dir_names`、`file_names`、`constructed`）则定义 `.harness-cli/` 下所有子目录和文件的规范路径，便于跨模块重命名。

## 关键文件

| 文件路径 | 职责 |
|----------|------|
| `src/utils/compare_versions.rs` | semver 版本号比较函数（支持 prerelease 的数字/字符串混合规则） |
| `src/constants/version.rs` | 编译时版本号常量：`VERSION`、`PACKAGE_NAME`（通过 `env!` 从 Cargo.toml 读取） |
| `src/constants/paths.rs` | 路径常量：`dir_names`（原子目录名）、`file_names`（原子文件名）、`constructed`（组合路径）、`get_workspace_dir`/`get_task_dir`/`get_archive_dir` 动态构造函数 |
| `src/constants/mod.rs` | 常量模块导出 |

## 核心功能

### Semver 版本比较

- **业务规则**: `compare_versions(a, b)` 返回 `Ordering`，支持完整 semver 规则（含预发布）
- **触发条件**: update 命令、启动时版本检查、迁移系统按版本排序
- **处理流程**:
  1. `split_version` 以第一个 `-` 分离基础版本和预发布标签
  2. `parse_base` 按 `.` 拆分基础版本为 `Vec<u64>`，逐位比较（未指定位视为 0）
  3. 基础版本相同时比较预发布：
     - 无预发布 > 有预发布（`1.0.0 > 1.0.0-beta`）
     - 两者都有 -> 按 `.` 拆分逐段比较
     - 段缺失者视为更短，排在前面（`1.0.0-beta < 1.0.0-beta.1`）
     - 每段区分数字段和字符串段：数字段 < 字符串段（semver 规则）
     - 数字段按 `u64` 比较，字符串段按字典序

### 版本常量

- **业务规则**: `VERSION` 和 `PACKAGE_NAME` 通过 `env!("CARGO_PKG_VERSION")`/`env!("CARGO_PKG_NAME")` 在编译时固定
- **触发条件**: 启动时、update 命令、版本显示时
- **处理流程**: 直接返回静态字符串引用

### 路径常量体系

- **业务规则**: 分三层定义路径
  - `dir_names`: 原子目录名（`WORKFLOW = ".harness-cli"`、`WORKSPACE = "workspace"`、`TASKS = "tasks"`、`SPEC = "spec"`、`SCRIPTS = "scripts"`、`KB = "kb"`、`KB_PRD = "prd"`、`KB_TECH = "tech"`、`ARCHIVE = "archive"`）
  - `file_names`: 原子文件名（`DEVELOPER = ".developer"`、`CURRENT_TASK = ".current-task"`、`TASK_JSON = "task.json"`、`PRD = "prd.md"`、`WORKFLOW_GUIDE = "workflow.md"`、`JOURNAL_PREFIX = "journal-"`）
  - `constructed`: 组合路径（`WORKFLOW = ".harness-cli"`、`WORKSPACE = ".harness-cli/workspace"`、`TASKS`、`SPEC`、`SCRIPTS`、`KB`、`KB_PRD = ".harness-cli/kb/prd"`、`KB_TECH = ".harness-cli/kb/tech"`、`DEVELOPER_FILE`、`CURRENT_TASK_FILE`、`WORKFLOW_GUIDE_FILE`）
- **触发条件**: 所有需要引用 `.harness-cli/` 内部路径的代码
- **处理流程**: 直接引用常量，确保全局路径一致性

### 动态路径构造

- `get_workspace_dir(developer)` -> `.harness-cli/workspace/<developer>`
- `get_task_dir(task_name)` -> `.harness-cli/tasks/<task_name>`
- `get_archive_dir()` -> `.harness-cli/tasks/archive`

## 数据流

```
启动时：
  CLI VERSION -> compare_versions -> 读取 .harness-cli/.version
  -> Greater: 提示 "harness-cli update"
  -> Less: 提示 "npm install -g <PACKAGE_NAME>"
  -> Equal: 静默

update 时：
  CLI VERSION -> compare_versions -> 项目版本
  -> 确定升级方向和迁移范围

迁移系统：
  get_all_migration_versions 使用 compare_versions 排序
  get_migrations_for_version 通过 compare_versions 过滤范围
```

## 业务规则

- 预发布版本低于正式版本：`0.3.0-beta.1 < 0.3.0`
- 预发布标签按字母序排列：`alpha < beta < rc`
- 相同预发布标签的数字后缀按数值比较：`beta.1 < beta.2 < beta.16`
- 数字段和字符串段混合时：数字段 < 字符串段
- 预发布段缺失者排在前面：`1.0.0-beta < 1.0.0-beta.1`
- 基础版本位数不同时，缺失位视为 `0`：`1.0 == 1.0.0`
- 所有 `constructed::*` 路径都以 `.harness-cli` 开头，使用正斜杠分隔符
- 当 CLI 版本高于项目版本时提示 `harness-cli update`
- 当 CLI 版本低于项目版本时提示用户升级 CLI（`npm install -g $PACKAGE_NAME`）

## 与其他模块的关系

| 模块 | 关系 |
|------|------|
| cli-commands | `main.rs` 启动时调用 `check_for_updates`，update 命令使用版本比较 |
| migration-system | 使用 `compare_versions` 对清单版本排序和过滤范围 |
| kb-system | 使用 `constructed::KB_PRD`/`KB_TECH` 路径常量 |
| file-management | 所有写文件路径通过 `constructed` 或 `dir_names` 构造 |
