# 系统全景

## 技术栈

| 类别 | 技术 | 版本 | 说明 |
|------|------|------|------|
| 语言 | Rust | 2021 edition | CLI binary 主体 |
| 语言 | Python | 3 (标准库) | 运行时脚本、hooks、multi-agent 编排 |
| 构建工具 | Cargo | - | release profile: `opt-level="z"`, `lto=true`, `strip=true` (Cargo.toml:48-51) |
| CLI 框架 | clap | 4 (derive) | 命令行解析 (src/main.rs:5) |
| 交互输入 | dialoguer | 0.11 | MultiSelect / Confirm 提示 |
| 终端 UI | colored | 2 | 彩色输出 |
| ASCII 艺术 | figlet-rs | 0.1 | 欢迎屏幕 |
| 序列化 | serde + serde_json | 1 / 1 | 配置解析、AI 工具注册表 |
| HTTP 客户端 | reqwest | 0.12 (blocking, json) | 远程模板下载 |
| 哈希 | sha2 + hex | 0.10 / 0.4 | 模板文件修改追踪（SHA256） |
| 嵌入资源 | rust-embed | 8 (include-exclude) | 编译时嵌入 14 个平台的模板 |
| 文件匹配 | glob | 0.3 | 路径模式 |
| 异步运行时 | tokio | 1 (full) | 声明但未实际使用，为未来预留 |
| 错误处理 | anyhow + thiserror | 1 / 2 | anyhow 广泛使用，thiserror 声明未用 |
| 正则 | regex | 1 | project_detector 中 package 检测 |
| 临时文件 | tempfile | 3 | 测试环境 |
| 二进制查找 | which | 7 | python/python3 检测 |
| Python 侧 | 仅标准库 | - | argparse / pathlib / json / subprocess / datetime / re / shutil（无外部依赖） |

**测试框架**
- Rust: `#[cfg(test)]` 内嵌 + `tests/regression.rs` 集成测试（~130 个）(src/configurators/mod.rs:172-473)
- Python: 无显式测试框架

## 核心组件

| 组件 | 一句话描述 | 入口文件 |
|------|-----------|----------|
| CLI Binary | 项目初始化、模板部署、版本管理、迁移 | src/main.rs:11-13 |
| commands::init | 三大子命令之一：初始化 `.harness-cli` 结构 + 配置 AI 平台 | src/commands/init.rs (src/main.rs:230-253) |
| commands::scan | 创建 KB (知识库) 目录结构 `.harness-cli/kb/{prd,tech}/` | src/commands/scan.rs (src/main.rs:255) |
| commands::update | 跨版本升级 + 文件迁移 | src/commands/update.rs (src/main.rs:256-270) |
| configurators | 13 个平台的 configurator，按 platform 分发部署 | src/configurators/mod.rs:100-116 |
| templates (API 层) | 强类型封装 rust-embed，暴露 get_all_commands/agents/hooks/settings | src/templates/claude.rs:37-52 |
| templates (资源层) | 实际的 Markdown/JSON/Python 文件，按平台分目录 | embedded/templates/{platform}/ |
| types::ai_tools | AITool 枚举 + AIToolConfig 注册表 (13 平台) | src/types/ai_tools.rs:14-29, 182-199 |
| migrations | 跨版本迁移引擎（rename/delete/safe-file-delete） | src/migrations/mod.rs |
| utils | project_detector / template_fetcher / template_hash / file_writer | src/utils/mod.rs |
| Python task.py | 任务 CRUD（create / archive / list / status / finish） | .harness-cli/scripts/task.py |
| Python multi_agent::plan | 规划阶段：启动 Plan Agent 产出 task.json | .harness-cli/scripts/multi_agent/plan.py |
| Python multi_agent::start | 调度阶段：创建 worktree + 启动 Dispatch Agent | .harness-cli/scripts/multi_agent/start.py |
| Python multi_agent::direct_merge | 完成阶段：commit + merge 到 target branch | .harness-cli/scripts/multi_agent/direct_merge.py |
| Python multi_agent::create_pr | 完成阶段：commit + push + 创建 Draft PR | .harness-cli/scripts/multi_agent/create_pr.py |
| Python multi_agent::cleanup | 清理阶段：删 worktree、从 registry 移除 agent | .harness-cli/scripts/multi_agent/cleanup.py |
| Python common/* | 运行时共享基础设施（见 cross-cutting.md） | .harness-cli/scripts/common/ |
| Claude Code hooks | SessionStart / PreToolUse / SubagentStop 回调 | embedded/templates/claude/settings.json:6-73 |

## 系统边界

**外部依赖**
- Git（通过 `run_git` 包装）：worktree、commit、push、merge
- GitHub CLI (`gh`)：create_pr.py 中创建 Draft PR
- Python 3：运行时所有脚本的宿主
- 网络：`reqwest` 从 GitHub 拉取 marketplace index（仅 init --template 时）
- AI CLI 工具（Claude Code / Cursor / iFlow / OpenCode / ...）：外部 agent 执行体

**输入输出接口**
- 输入：`cargo` 编译 → 单一二进制；用户通过 shell 调用 `harness-cli <subcommand>`
- 输出：在当前目录下生成/更新 `.harness-cli/`、`.claude/`、`.cursor/` 等平台配置目录
- 运行时输入：task.json（任务生命周期）、config.yaml（项目配置）、worktree.yaml（并行配置）
- 运行时输出：Git worktree、feature branch、PR URL、归档任务目录

**部署目标**
- 开发者工作站（macOS / Linux / Windows）
- 通过 `cargo install` 或预编译二进制分发
- 单一文件，无运行时依赖（除 Python 3，由 hooks 脚本运行时需要）
