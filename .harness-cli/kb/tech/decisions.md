# 架构决策记录

## ADR-001: 使用 rust-embed 嵌入模板资源

- **状态**: 已采纳
- **日期**: 项目初始
- **背景**: CLI 需要将 14 个 AI 平台的命令、agent、hook、settings 模板打包进二进制，让用户 `cargo install` 后无需额外下载即可使用。需要支持运行时枚举、按模式过滤、多目录隔离。
- **选项**:
  1. `include_str!` 宏 -- 优点：零依赖、编译时检查；缺点：只能按文件逐个指定路径，无法枚举目录，新增文件需手工维护清单
  2. `build.rs` 构建脚本 -- 优点：完全灵活；缺点：增加构建复杂度，运行时仍需自己实现枚举
  3. `rust-embed` crate -- 优点：derive 宏支持 `#[folder]` 声明 + `#[exclude]` 过滤；缺点：引入依赖
- **决定**: 选 `rust-embed`，每个平台一个 `#[derive(Embed)]` 结构体 (src/templates/extract.rs:18-120)，通过 `list_files::<T>()` 在运行时动态枚举
- **后果**:
  - ✅ 新增 `hc:archive` 命令只需往 `embedded/templates/claude/commands/hc/` 丢 `.md` 文件，编译自动发现
  - ✅ 14 个平台通过 `ClaudeTemplates`、`CursorTemplates` 等强类型结构体隔离，互不干扰
  - ✅ `#[exclude = "*.ts"]` 等模式避免打包编译产物
  - ⚠ 增加一个编译期依赖

---

## ADR-002: src/templates 作为 API 层，embedded/templates 作为资源层

- **状态**: 已采纳
- **日期**: 项目初始
- **背景**: 如果 configurator 直接调 rust-embed 原始 API，每个 configurator 都要重复写 `list_files()` + 过滤 + 字符串拼接逻辑；类型层面也无法区分 CommandTemplate 和 AgentTemplate
- **选项**:
  1. configurator 直接读 rust-embed 原始资源
  2. 引入一个 API 层封装类型 + getter 函数
- **决定**: 建立 `src/templates/{platform}.rs` 作为 API 层（src/templates/claude.rs:8-34），暴露：
  - `CommandTemplate`、`AgentTemplate`、`HookTemplate`、`SettingsTemplate` 强类型
  - `get_all_commands()`、`get_all_agents()`、`get_all_hooks()`、`get_settings_template()` getter
- **后果**:
  - ✅ configurator 只需关心"我要什么类型的模板"，不关心底层实现
  - ✅ 测试可以直接 mock/check 这些 API（src/templates/claude.rs:104-194 有 15+ 测试）
  - ✅ `collect_templates()` 可以统一生成 `路径 → 内容哈希` 映射，用于 update 命令追踪用户修改 (src/configurators/claude.rs:30-58)

---

## ADR-003: Rust binary + Python scripts 混合架构

- **状态**: 已采纳
- **日期**: 项目初始
- **背景**: 需要兼顾"单一可执行文件的分发便利性"和"运行时可扩展、可 hack 的灵活性"。CLI 初始化是一次性的重 I/O 操作，而运行时的任务编排、hook 执行是高频的、需要与 AI CLI（Claude Code / Cursor）紧密集成的
- **选项**:
  1. 纯 Rust -- 优点：性能、类型安全、单一二进制；缺点：用户不能直接修改脚本，AI hook 集成复杂
  2. 纯 Python -- 优点：灵活；缺点：分发复杂，依赖 Python 环境才能启动
  3. Rust binary + Python scripts -- Rust 负责初始化和文件部署，Python 负责运行时逻辑
- **决定**: 混合架构
  - **Rust**: `commands::init` / `scan` / `update`，部署时把 `.harness-cli/scripts/` 写到项目目录
  - **Python**: 任务 CRUD (`task.py`)、工作流编排 (`multi_agent/*`)、hook 执行 (`hooks/*`)
- **后果**:
  - ✅ 用户可以直接编辑 `.harness-cli/scripts/` 修改行为，无需重编译
  - ✅ Claude Code / Cursor 的 hook 机制原生支持 Python 脚本，集成简单
  - ✅ Python 脚本**零外部依赖**（仅标准库），避免 `pip install` 步骤
  - ⚠ 通信靠文件系统（task.json / registry.json），没有进程内 API
  - ⚠ 一次升级需要同时更新 Rust 代码和 embedded 里的 Python 脚本
  - ⚠ `src/templates/claude.rs` 这类文件存在，而 `.harness-cli/scripts/` 也存在，新手容易混淆二者的角色

---

## ADR-004: 平台 configurator 互相独立、不 DRY

- **状态**: 已采纳
- **日期**: 项目初始
- **背景**: 13 个 AI 平台的配置结构差异很大——Claude Code 有 commands+agents+hooks+settings，Codex/Kiro 是 skills，Antigravity/Windsurf 是 workflows，Copilot 是 GitHub hooks
- **选项**:
  1. 提取抽象 trait 或基类强制统一
  2. 每个平台独立一个 configurator，手工复制相似逻辑
- **决定**: 独立 configurator (src/configurators/{platform}.rs)，通过 `configure_platform()` 的 match 分发 (src/configurators/mod.rs:100-116)
- **后果**:
  - ✅ 新增平台成本低：加一个文件 + 一个 match 分支，不影响其他平台
  - ✅ 一个平台的 bug 不会污染其他平台
  - ⚠ 平台间相似逻辑（如 copy_embedded_dir 调用）有重复
  - ⚠ 修改"所有平台都该做的事"需要改 13 次（例如加一个全局 placeholder）

---

## ADR-005: 使用 git worktree 做并行任务隔离

- **状态**: 已采纳
- **日期**: multi-agent 功能开发期
- **背景**: 支持多个 agent 并行在不同 feature branch 上工作。单个仓库只能 checkout 一个分支，会互相冲突；完全 clone 又太重
- **选项**:
  1. Shallow clone -- 每个 task 独立仓库，磁盘浪费、慢
  2. In-place checkout + 切换 -- 无法真正并行
  3. `git worktree` -- 共享 `.git`，每个 worktree 有独立的工作树和 HEAD
- **决定**: 用 `git worktree add <path> <branch>` 为每个 task 创建独立工作目录 (multi_agent/start.py:L358-372)
- **后果**:
  - ✅ 多个 agent 可同时编辑不同分支，互不干扰
  - ✅ 轻量：共享 git objects，只复制工作树
  - ✅ `git worktree remove` 自动清理
  - ⚠ worktree 不能 checkout 已被主仓或另一 worktree 占用的分支 —— direct_merge.py 需要特殊处理：切换到主仓去 merge (src/multi_agent/direct_merge.py:213-310)
  - ⚠ 隐含约束：worktree 的 `.harness-cli/` 是 `start.py` 手工 `shutil.copytree` 复制的（ADR-006）—— 导致 task.json 状态同步 bug

---

## ADR-006: task 目录复制到 worktree（而非 git 追踪）

- **状态**: 已采纳（已知引入 bug）
- **日期**: multi-agent 功能开发期
- **背景**: 任务创建后 `task.json` 还未 commit，但 agent 需要在 worktree 中读它
- **选项**:
  1. 要求任务创建后立即 commit 再创建 worktree
  2. 用 `shutil.copytree` 把 task 目录复制到 worktree
- **决定**: 方案 2，在 `start.py:405-412` 物理复制 `.harness-cli/tasks/<task>/` 到 worktree
- **后果**:
  - ✅ 任务不需要 commit 就能启动 agent
  - ⚠ **已知 bug**：worktree 的 `.harness-cli/` 子目录触发 `get_repo_root()` 把 worktree 当作 repo root，导致 `direct_merge.py` / `create_pr.py` 更新的是 worktree 副本的 task.json，而不是主仓。此 bug 在 2026-04-11 被修复：新增 `get_main_repo_root()` 函数，在 Step 6 额外同步到主仓 (src/common/paths.py, multi_agent/direct_merge.py:369-381, multi_agent/create_pr.py:605-625)
  - 📝 教训：复制共享状态到隔离副本时，**必须**设计"回写同步"机制

---

## ADR-007: task.json 状态机 + next_action 数组驱动流程

- **状态**: 已采纳
- **日期**: multi-agent 功能开发期
- **背景**: 多 agent 协作需要明确的流程定义，且要支持不同任务类型走不同流程（例如 bug-fix vs feature）
- **选项**:
  1. 硬编码状态机在 Python 代码中
  2. 在 task.json 中用数组外化流程步骤
- **决定**: `next_action: list[{phase, action}]` + `current_phase: int` 组合
  - 预定义 action：`implement` / `check` / `finish` / `create-pr` / `direct-merge`
  - Plan Agent 创建任务时决定 action 序列
  - Dispatch Agent 按 `current_phase` 索引 `next_action` 执行
- **后果**:
  - ✅ 流程可动态扩展：不同任务类型可以有不同的 next_action 序列
  - ✅ 可观测：每个 agent 都知道"3/4"的进度
  - ✅ 可恢复：agent 挂掉后可以从 `current_phase` 继续
  - ✅ 决策透明：流程定义外移到 task.json，不硬编码
  - ⚠ 用户可以手工修改 task.json 造成状态不一致

---

## ADR-008: 零外部依赖的 Python 脚本

- **状态**: 已采纳
- **日期**: 项目初始
- **背景**: AI CLI 工具（Claude Code）的 hook 机制是在受限环境中执行 Python 脚本。用户不期望先 `pip install` 才能用
- **选项**:
  1. 允许使用 PyYAML、requests 等常见库
  2. 只用 Python 标准库
- **决定**: `.harness-cli/scripts/common/` 中所有模块只导入 Python 3 标准库
  - YAML 解析：`.harness-cli/scripts/common/worktree.py` 手工实现简单 YAML 解析
  - JSON：`json` 标准库
  - HTTP：如有需要用 `urllib`（未见使用）
- **后果**:
  - ✅ 部署简单：任何有 Python 3 的环境都能运行，无需虚拟环境
  - ✅ 启动快：不用加载第三方库
  - ⚠ YAML 解析是简化版，不支持完整语法（只够项目自己的 config.yaml / worktree.yaml 使用）
  - ⚠ 复杂依赖需求（如 gRPC、数据库）不在支持范围内

---

## ADR-009: 模板修改追踪用 SHA256 哈希

- **状态**: 已采纳
- **日期**: update 命令引入时
- **背景**: `harness-cli update` 需要判断：用户是否修改了某个模板文件？如果修改了，不应无声覆盖
- **选项**:
  1. 时间戳对比 -- 容易误判（git checkout 会重写时间）
  2. 保存一份旧模板内容 -- 磁盘浪费
  3. SHA256 哈希对比
- **决定**: 计算 `SHA256(embedded_content)` 存到 `.harness-cli/.hashes.json`（或类似位置），update 时对比
- **后果**:
  - ✅ 精确识别用户修改
  - ✅ 磁盘开销小（每个文件只存 32 字节）
  - ✅ `collect_templates()` 函数统一计算，实现简单 (src/configurators/claude.rs:30-58)

---

## ADR-010: 跨平台 Python 命令检测与 placeholder 替换

- **状态**: 已采纳
- **日期**: 项目初始
- **背景**: Claude Code 的 settings.json hooks 需要指定 `python3 xxx.py`，但 Windows 上通常命令是 `python`
- **选项**:
  1. 生成两套 settings.json
  2. 用 placeholder + 运行时检测
- **决定**: `{{PYTHON_CMD}}` placeholder，init 时 `resolve_placeholders()` 根据 `cfg!(windows)` 替换 (src/configurators/shared.rs:14-19)
- **后果**:
  - ✅ 单一模板，跨平台自动适配
  - ✅ 扩展简单：未来加 `{{NODE_CMD}}` 等同理
  - ⚠ 只在 `settings.json` 中替换，其他文件不处理（placeholder_filename 明确限定）

---

## ADR-011: 迁移系统与 manifest 驱动升级

- **状态**: 已采纳
- **日期**: 多版本并存时期
- **背景**: 跨版本升级需要重命名/删除旧文件，且不能误删用户的修改
- **选项**:
  1. 靠文档让用户手工操作
  2. 硬编码迁移逻辑在 Rust 代码中
  3. 每个版本一个 manifest JSON 描述迁移
- **决定**: `embedded/manifests/{version}.json` 存储每个版本的迁移清单，操作类型：
  - `rename` / `rename-dir`
  - `delete`
  - `safe-file-delete`（删前对比哈希，不匹配不删）
- **后果**:
  - ✅ 迁移是声明式的，review 清晰
  - ✅ `safe-file-delete` 保护用户修改
  - ⚠ 每个版本发布都要写 manifest，增加了流程步骤

---

## ADR-012: Claude Code 优先，其他平台次优先级

- **状态**: 隐含采纳
- **日期**: 项目演进
- **背景**: 虽然支持 13 个平台，但命令数量在 embedded 里分布不均
- **观察**: 
  - claude: 17 个 hc 命令
  - opencode: 15 个
  - iflow: 13 个
  - codebuddy/cursor/windsurf: 12 个
  - codex/copilot/gemini/kilo/kiro/qoder: **0 个**
  - `task-dashboard`、`archive`、`scan-kb-tech` 等新命令只在 claude 平台存在
- **决定**: 不强制要求所有平台同步所有命令；每个命令按需在部分平台实现
- **后果**:
  - ✅ 降低新命令的交付成本（先在 claude 上线，验证后再推广）
  - ✅ 不同平台的能力可以差异化
  - ⚠ 用户需要知道自己用的平台是否支持某个命令
  - ⚠ 缺少"命令矩阵"文档，用户难以发现哪些命令可用

---

## ADR-013: init 和 scan 是交互式的，支持 --yes/--force 非交互模式

- **状态**: 已采纳
- **日期**: 项目初始
- **背景**: 默认用交互式确认可以防止误操作，但 CI/CD 或脚本调用需要非交互模式
- **选项**:
  1. 默认非交互，有 `--interactive` 开启
  2. 默认交互，有 `--yes` / `--force` 关闭
- **决定**: 方案 2（与 `rm`、`apt-get` 等工具一致）
- **后果**:
  - ✅ 符合 Unix 传统习惯
  - ⚠ **已知 bug**：在非 TTY 环境（如子进程捕获 stdout）下，不传 `--yes` 会报 "not a terminal" 错误，需要显式传 `--yes --force`（在本次 init --claude 过程中遇到）

---

## ADR-014: task archive 强制 KB 状态 gate（kb_status 字段）

- **状态**: 已采纳
- **日期**: 2026-04-14
- **背景**: 早期 `after_archive` hook 只是 `echo` 一条"请检查 KB"提醒，事实上被人和 AI 普遍忽略——journal 与 KB 在多次 task archive 后持续漂移。需要一种结构性手段强制每次 archive 都明确声明"KB 是否同步"。
- **选项**:
  1. **软提醒**（现状延续）：在 `after_archive` hook 打印提示，由人判断是否需要 `/hc:scan-kb`。
  2. **基于文件路径的自动判断**：扫描 task 关联的 commits 改动文件，落在业务代码路径（`src/`、`embedded/` 等）白名单则要求 KB 更新。
  3. **三态字段 + archive gate**（现选）：在 task.json 加 `kb_status` 三态字段（`needed` / `updated` / `not_required`），`archive` 命令硬阻塞当值为 `needed`；由 AI/用户通过 `task.py mark-kb` 显式流转状态。
- **决定**: 方案 3
- **理由**:
  - 方案 1 已被证明失效（改动前本仓库 journal 和 KB 都处于严重漂移状态）
  - 方案 2 的路径白名单天然脆弱（一次重构就可能漏判；测试目录 vs 业务目录的边界不清晰；跨语言项目更难处理）
  - 方案 3 把"是否需要更新 KB"这个**模型判断**放回模型——AI 在 `/hc:finish-work` 或 `/hc:scan-kb` 环节自己决定 task 的 `kb_status` 应为 `updated` 还是 `not_required`
- **实现**:
  - 字段：`task.json.kb_status`（默认 `needed`，见 `cmd_create` 在 task_store.py）
  - CLI：`task.py mark-kb <status> [<task>]`（接受连字符形式 `not-required` 并规范化）
  - Gate：`cmd_archive` 起始处读 `kb_status`，若为 `needed` 打印错误 + exit 1
  - Legacy 兼容：`pre_data.get("kb_status", "needed")` 兜底，旧任务也被 gate
- **后果**:
  - ✅ 结构性保证 KB 与代码不漂移
  - ✅ 判断逻辑归给模型，避免硬规则维护
  - ⚠ 增加用户学习成本（需要知道 `mark-kb` 命令）
  - ⚠ 没有逃生阀（无 `--force`），紧急情况下只能手动编辑 task.json（这是刻意决定：逃生阀会被滥用）

---

## ADR-015: Session 记录从 pull-based 改为 push-based（task.finish 自动触发）

- **状态**: 已采纳
- **日期**: 2026-04-13
- **背景**: 早期设计是 pull-based：用户/AI 需要主动跑 `/hc:record-session` → `add_session.py --title ... --commit ...`。实际效果是**没人跑**：journal 文件永远是空的，workspace/{dev}/index.md 和全局 workspace/index.md 都无数据。这是个设计失败。
- **选项**:
  1. **保留 pull-based，加强提示**：在 `/hc:finish-work` 命令里更醒目地提醒运行 `/hc:record-session`。
  2. **Cron / 定时扫描**：周期性扫描 git log 推断 session。
  3. **Git post-commit hook**：每次 commit 触发 session 记录。
  4. **Task 生命周期事件 push**（现选）：`task.py finish` 时自动调用 session 记录 + 全局 index 刷新；`task.py archive` 时只刷新全局 index（session 在 finish 时已记录）。
- **决定**: 方案 4
- **理由**:
  - 方案 1 只是"加强提醒"，没有改变"用户必须记住去跑"的根本问题——证据是本仓库在做这个 ADR 之前就已经有多次"忘记手动记录"的累积
  - 方案 2 侵入性强，周期性任务难以在多环境稳定运行（CI / 多机协作）
  - 方案 3 粒度太细：不是每次 commit 都对应一次有意义的 session；会产生噪音
  - 方案 4 粒度刚好：**task 是有语义的工作单元**，它的完成/归档是自然的"记录节点"
- **实现**:
  - Orchestrator: `_auto_record_session(task_json_path, repo_root)`（位于 task.py，不在 task_store.py）
  - 两步副作用：session 记录（`add_session_from_task`）+ 全局索引刷新（`refresh_global_workspace_index`），相互独立 try/except
  - SystemExit 必须被捕获（因 `ensure_developer` 未初始化时 `sys.exit`）
  - 非阻塞：任一副作用失败只打印 `[WARN]`，不中断 `finish` / `archive` 主流程
  - `auto_commit=False`（finish 路径），与 CLI 手动路径的 `auto_commit=True` 相反
- **后果**:
  - ✅ journal 与 index 从死文件变为活文件（实测：从 0 session 变成多 session 自动累积）
  - ✅ 开发者无需记住额外命令
  - ⚠ `finish` 执行开销略有增加（两个子副作用），但都在毫秒级
  - ⚠ 失败是静默的（只打 warn），需通过日志观察而非阻塞式报错——这是刻意设计（不能让索引写不动拦住 finish）
