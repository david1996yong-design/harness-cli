# CLI 命令系统

> 提供 harness-cli 的三个核心子命令：init、scan、update

## 模块概述

CLI 命令系统是 harness-cli 的入口层，负责解析命令行参数并调度对应的业务逻辑。使用 `clap` 库实现参数解析，通过 `Commands` 枚举定义三个子命令。启动时会检查项目版本与 CLI 版本的差异并提示用户升级。

## 关键文件

| 文件路径 | 职责 |
|----------|------|
| `src/main.rs` | CLI 入口点，定义 `Cli` 和 `Commands` 枚举，实现启动时版本检查 `check_for_updates` |
| `src/commands/mod.rs` | 命令模块导出（init、scan、update） |
| `src/commands/init.rs` | `init` 命令实现：交互式设置项目，创建 `.harness-cli/` 工作流，配置平台，可选下载远程模板，创建 bootstrap 任务 |
| `src/commands/scan.rs` | `scan` 命令实现：创建 `kb/prd/` 和 `kb/tech/` 目录并写入 index 和 module 模板 |
| `src/commands/update.rs` | `update` 命令实现：比对模板哈希、分类变更、按冲突策略写入、执行迁移清单 |

## 核心功能

### init 命令

- **业务规则**: 在当前项目中创建 `.harness-cli/` 工作流目录、AI 平台配置（如 `.claude/`、`.cursor/`）、spec 模板和 bootstrap 任务
- **触发条件**: 用户运行 `harness-cli init [选项]`
- **处理流程**:
  1. 打印 ASCII banner 和欢迎信息
  2. 读取 `HTTPS_PROXY` 等环境变量设置代理
  3. 根据 `--force`/`--skip-existing` 设置全局 `WriteMode`
  4. 从 git config 读取开发者名字，否则交互提示输入
  5. 检测项目类型（Frontend/Backend/Fullstack）
  6. 解析可选的 `--registry` 自定义模板源
  7. 检测 monorepo 结构，交互式启用并为各包单独选择模板
  8. 交互式或通过 CLI flag 选择 AI 平台
  9. 单仓模式下可选选择远程 spec 模板（市场列表或自定义 registry）
  10. 下载远程模板（如选择）
  11. 创建基础目录结构（`.harness-cli/workspace`、`tasks`、`spec`、`scripts`）
  12. 写入 `.version` 文件
  13. 为选中的平台调用 `configure_platform`
  14. 创建 `AGENTS.md` 根文件
  15. 调用 `initialize_hashes` 计算所有受管文件的 SHA256 哈希
  16. 运行 `scripts/init_developer.py` 写入开发者身份
  17. 创建 `00-bootstrap-guidelines` 任务（包含 `task.json` 和 `prd.md`）并设为当前任务
  18. 打印 "What We Solve" 痛点与方案列表

### scan 命令

- **业务规则**: 创建知识库的目录骨架（kb/prd 和 kb/tech），写入 index.md 和 `_module-template.md` 模板文件
- **触发条件**: 用户运行 `harness-cli scan`
- **处理流程**:
  1. 确认 `.harness-cli/` 目录存在，否则打印错误并退出
  2. 如带 `--force` 则设置全局 `WriteMode::Force`
  3. 创建 `kb/prd/` 目录，从嵌入模板写入 `index.md` 和 `_module-template.md`
  4. 创建 `kb/tech/` 目录，从嵌入模板写入 `index.md` 和 `_module-template.md`
  5. 打印成功提示并提示用户运行 `/hc:scan-kb` 和 `/hc:scan-kb-tech`

### update 命令

- **业务规则**: 将项目 `.harness-cli/` 和平台配置目录更新到当前 CLI 版本，支持哈希比对、冲突策略和迁移清单
- **触发条件**: 用户运行 `harness-cli update [选项]`
- **处理流程**:
  1. 读取 `.harness-cli/.version` 获取项目版本
  2. 对比 CLI 版本，除非带 `--allow-downgrade` 否则拒绝降级
  3. 定义 protected_paths（`workspace/`、`tasks/`、`spec/`、`.developer`、`.current-task`）
  4. 收集受管路径下的现有文件和新的嵌入模板内容
  5. 通过哈希比对分类文件：new、unchanged、auto_update、changed（用户已修改）、user_deleted
  6. 处理 SafeFileDelete 迁移项：只删除内容匹配 `allowed_hashes` 的未被保护的文件
  7. 根据 `--force`/`--skip-all`/`--create-new` 或交互提示处理 changed 文件
  8. 可选执行版本范围内的迁移清单（`--migrate` 时或启动时确认）
  9. 更新 `.version` 和 `.template-hashes.json`

### 启动时版本检查

- **业务规则**: 每次运行时检测 CLI 版本和项目版本差异，打印升级提示
- **触发条件**: 程序启动时，仅当当前目录下存在 `.harness-cli/` 时执行
- **处理流程**:
  1. 读取 `.harness-cli/.version` 文件
  2. 调用 `compare_versions` 比较 CLI 版本
  3. `Greater` -> 黄色提示 `harness-cli update`
  4. `Less` -> 黄色提示用户升级 CLI（`npm install -g <package>`）
  5. `Equal` -> 静默

## 数据流

```
用户输入 CLI 参数
  -> clap 解析为 Commands 枚举（Init/Scan/Update）
  -> main.rs 将子命令参数转换为 Options 结构体
  -> 调度到 commands::{init,scan,update}::*()
  -> 命令函数调用 configurators/templates/utils 执行实际操作
  -> 结果输出到终端；错误通过 anyhow 传回 main 并打印
```

## 业务规则

- init 命令支持 13 个 AI 平台的独立选择，默认选中 Claude Code 和 Cursor
- init 命令的 `--yes` flag 跳过所有交互提示使用默认值
- init 命令的 `--force` flag 覆盖已存在的文件，`--skip-existing` 跳过已存在文件；二者不能同时使用
- init 命令的 `--monorepo`/`--no-monorepo` 互斥，用于强制启用或跳过 monorepo 检测
- init 命令的 `--template` 和 `--registry` 可组合使用，用于从自定义仓库下载 spec 模板
- update 命令通过哈希比较检测用户已修改的文件，避免覆盖用户自定义内容
- update 命令的 protected_paths 列表始终跳过：`workspace/`、`tasks/`、`spec/`、`.developer`、`.current-task`
- update 命令的 `--create-new` 对冲突文件生成 `.new` 副本而不是直接覆盖
- scan 命令要求 `.harness-cli/` 已存在，否则提示先运行 init
- 所有命令在启动时都会触发 `check_for_updates` 版本检查（仅当 `.harness-cli/` 存在）

## 与其他模块的关系

| 模块 | 关系 |
|------|------|
| platform-configurators | init 命令调用 configurators 为选中的平台创建配置 |
| template-system | init/scan/update 命令通过 markdown 和 harness_cli 模板读取嵌入内容 |
| migration-system | update 命令调用迁移系统获取迁移项并执行 |
| file-management | 所有命令通过 `file_writer::write_file` 写入文件 |
| project-detection | init 命令调用项目检测和 monorepo 检测 |
| version-management | update 命令和启动检查使用 `compare_versions` |
| template-fetcher | init 命令可选从远程下载 spec 模板；update 命令使用 template_hash 比对哈希 |
| kb-system | scan 命令是 KB 目录创建的入口 |
