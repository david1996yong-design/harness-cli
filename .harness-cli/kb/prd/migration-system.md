# 版本迁移系统

> 在 CLI 版本升级时自动处理文件重命名、删除等破坏性变更

## 模块概述

迁移系统管理跨版本升级时的文件变更。每个版本有一个 JSON 格式的迁移清单（manifest），描述需要执行的文件操作（重命名/删除/安全删除）。清单在编译时通过 `rust-embed` 嵌入二进制文件，update 命令在检测到版本差异时自动执行适用的迁移。类型定义单独放在 `types/migration.rs`，迁移加载和聚合逻辑放在 `migrations/mod.rs`。

## 关键文件

| 文件路径 | 职责 |
|----------|------|
| `src/migrations/mod.rs` | 迁移清单的加载、缓存、版本范围查询和元数据聚合（`load_manifests`、`get_migrations_for_version`、`get_migration_summary`、`get_migration_metadata`、`get_all_migrations` 等） |
| `src/types/migration.rs` | 迁移相关类型定义：`MigrationType`、`MigrationItem`、`MigrationManifest`、`ClassifiedMigrations`、`MigrationResult`、`MigrationAction`、`TemplateHashes` 等 |
| `embedded/manifests/*.json` | 各版本的迁移清单文件（编译时嵌入） |

## 核心功能

### 迁移清单加载

- **业务规则**: 从嵌入的 JSON 文件解析迁移清单，按版本号索引缓存
- **触发条件**: 首次调用迁移相关函数时（通过 `OnceLock` 惰性初始化）
- **处理流程**: 遍历 `MigrationManifests` 嵌入资源 -> 仅处理 `.json` 文件 -> 解析 `MigrationManifest` -> 存入 `HashMap<version, MigrationManifest>`；解析失败时打印 warning 不中断

### 版本范围查询

- **业务规则**: `get_migrations_for_version(from, to)` 返回版本号严格大于 `from` 且小于等于 `to` 的所有迁移项，按版本号排序
- **触发条件**: update 命令检测到版本差异时
- **处理流程**: 排序版本 -> 过滤范围 -> 展开每个 manifest 的 `migrations` 列表

### 迁移摘要与聚合

- **业务规则**:
  - `has_pending_migrations(from, to)` 返回是否存在待执行迁移
  - `get_migration_summary(from, to)` 返回分类计数（`renames`、`deletes`、`safe_file_deletes`）
  - `get_migration_metadata(from, to)` 聚合范围内的 changelog、breaking、recommend_migrate 标志和 migration_guides
  - `get_all_migration_versions()` 返回所有已注册版本（排序）
  - `get_all_migrations()` 返回所有迁移项（不论版本）
- **触发条件**: update 命令展示迁移信息、检测孤立迁移时

### 迁移操作类型

- **Rename**: 重命名单个文件
- **RenameDir**: 重命名目录
- **Delete**: 直接删除文件
- **SafeFileDelete**: 仅当文件内容匹配 `allowed_hashes` 列表中的已知哈希时才删除（避免误删用户修改过的文件）

### 迁移分类（在 update 命令中）

- **业务规则**: 根据文件状态将迁移分为 Auto（可自动执行）、Confirm（需确认）、Conflict（冲突）、Skip（跳过）
- **触发条件**: update `--migrate` 执行时
- **处理流程**: 检查源文件是否存在、目标文件是否存在、文件是否被用户修改（通过哈希比较）

### 元数据和变更日志

- **业务规则**: `get_migration_metadata(from, to)` 汇总 changelog、breaking 状态、migrate 推荐和 migration_guides（含每个版本的 AI 指令）
- **处理流程**: 遍历版本范围内的 manifest -> 聚合 `changelog`、设置 `breaking`/`recommend_migrate` OR 聚合 -> 追加 `migration_guide` 为 `MigrationGuideEntry`

## 数据流

```
embedded/manifests/*.json
  -> 编译时嵌入 MigrationManifests
  -> load_manifests() -> OnceLock 缓存 HashMap<String, MigrationManifest>
  -> get_migrations_for_version(from, to) 过滤版本范围
  -> update 命令按类型分类为 Auto/Confirm/Conflict/Skip
  -> 执行迁移操作（重命名/删除文件）
  -> 更新 .template-hashes.json
```

## 业务规则

- 迁移清单必须包含 `version` 和 `migrations` 字段
- 可选字段：`description`、`changelog`、`breaking`、`recommendMigrate`、`migrationGuide`、`aiInstructions`（camelCase 反序列化）
- `MigrationItem` 类型反序列化为 kebab-case：`rename` / `rename-dir` / `delete` / `safe-file-delete`
- `SafeFileDelete` 只在 `allowed_hashes` 匹配时执行，保护用户自定义内容
- 没有 `allowed_hashes`（或为空）的 SafeFileDelete 一律 SkipModified
- 版本范围为左开右闭：`(from, to]`
- 破坏性变更（`breaking: true`）的版本会推荐用户执行 `--migrate`
- 迁移按版本号排序执行（使用 semver `compare_versions`）
- JSON 解析失败不中断，只打印 warning

## 与其他模块的关系

| 模块 | 关系 |
|------|------|
| cli-commands | update 命令调用迁移系统获取迁移项和元数据 |
| version-management | 使用 `compare_versions` 判断版本范围 |
| template-fetcher | SafeFileDelete 使用 `template_hash::compute_hash` 比对哈希 |
| file-management | update 命令通过 `write_file` 或直接 `fs::rename`/`fs::remove_file` 执行迁移 |
