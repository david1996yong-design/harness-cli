# 横切关注点

## 错误处理与传播

### Rust 侧

**错误类型**: 统一使用 `anyhow::Result<T>`

- 所有命令入口函数签名都是 `pub fn foo(...) -> Result<()>`
- 通过 `?` 运算符自动向上传播
- 用 `.context("failed to X")` 添加上下文 (src/commands/init.rs 等)
- 用 `anyhow!("message")` 或 `bail!("message")` 直接构造错误 (src/utils/template_fetcher.rs:156-171)

**示例模式**:
```rust
use anyhow::{anyhow, Context, Result};

pub fn init(options: InitOptions) -> Result<()> {
    let project_type = detect_project_type(&cwd)
        .context("Failed to detect project type")?;
    
    if parts.len() != 2 {
        return Err(anyhow!("Invalid registry source: {}", source));
    }
    // ...
}
```

**thiserror**: Cargo.toml 声明了 `thiserror = "2"` 但项目中尚未使用，推测为未来引入自定义错误类型时预留

**用户可见错误 vs 内部错误**: 没有明确区分。所有错误通过 `main()` 的 `?` 最终由 clap 或 anyhow 打印到 stderr。交互式命令（init / scan）会在失败时打印彩色提示（如 "Error: IO error: not a terminal"）

### Python 侧

**错误类型**: 手写简单 try/except，无自定义 Exception 层次

**共享日志函数** (`.harness-cli/scripts/common/log.py`):
```python
def log_error(msg: str) -> None:
    print(f"{Colors.RED}[ERROR]{Colors.NC} {msg}")

def log_warn(msg: str) -> None:
    print(f"{Colors.YELLOW}[WARN]{Colors.NC} {msg}")
```

**配置加载的静默降级** (`common/config.py`, `common/worktree.py`):
```python
def _load_config(repo_root):
    try:
        content = config_file.read_text(encoding="utf-8")
        return parse_simple_yaml(content)
    except (OSError, IOError):
        return {}  # 失败时返回空 dict，不中断流程
```

这个模式在整个 common 层广泛使用：**配置/状态文件读取失败 = 返回默认值**，避免单点故障阻塞 agent 启动

**multi_agent 脚本的错误处理**: 失败时显式 `log_error(...)` + `return 1`，让调用方（subprocess）通过 exit code 判断。无异常栈传播给 AI agent

**KB Status Gate 的错误路径** (task_store.py `cmd_archive`): 归档前读 task.json 的 `kb_status`；若为 `needed` 立即打印红色 `Error: cannot archive '<name>' — kb_status is 'needed'` + 两条修复路径指引，返回 exit 1，**硬阻塞**（不像其他 Python 错误多是 soft degradation）。这是有意为之：KB 漂移是产品级风险，不能 silent pass。

**Task 生命周期副作用的错误隔离** (`_auto_record_session` in task.py): session 记录（step 1）和全局索引刷新（step 2）用**各自独立**的 try/except 包裹（step 2 的 catch 在 `refresh_global_workspace_index` 内部），一方失败不会影响另一方。两处都 catch `(Exception, SystemExit)` —— `SystemExit` 来自 `ensure_developer()` 未初始化时的 `sys.exit`，若不 catch 会穿透 `finish` 流程导致 `.current-task` 无法清除、用户卡死。

---

## 日志管道

### Rust 侧

**无独立日志库**（Cargo.toml 中没有 log / tracing / env_logger）

输出机制:
- `println!` / `eprintln!` 直接打印到 stdout/stderr
- `colored` crate 提供 `.red()`、`.green()`、`.yellow()` 等方法美化输出
- 典型示例 (src/main.rs:155-163):
```rust
println!(
    "{}",
    format!(
        "\n  Harness CLI update available: {} -> {}",
        project_version, cli_version
    )
    .yellow()
);
```

**无结构化日志**，无日志级别（debug/info/warn/error），无文件输出——纯 CLI 工具的"给人看"输出

### Python 侧

**统一彩色前缀日志** (`.harness-cli/scripts/common/log.py`):

```python
class Colors:
    RED = "\033[0;31m"
    GREEN = "\033[0;32m"
    YELLOW = "\033[1;33m"
    BLUE = "\033[0;34m"
    NC = "\033[0m"

def log_info(msg):    print(f"{Colors.BLUE}[INFO]{Colors.NC} {msg}")
def log_success(msg): print(f"{Colors.GREEN}[SUCCESS]{Colors.NC} {msg}")
def log_warn(msg):    print(f"{Colors.YELLOW}[WARN]{Colors.NC} {msg}")
def log_error(msg):   print(f"{Colors.RED}[ERROR]{Colors.NC} {msg}")
```

所有 `multi_agent/*.py` 都导入这些函数，前缀风格统一

### Agent 日志（运行时）

- 每个 worktree 根目录的 `.agent-log` 文件存储后台 agent 的 stream-json 输出
- 由 `start.py` 创建，通过 `popen_kwargs["stdout"] = log_f` 重定向
- 格式：每行一个 JSON 事件（`{"type": "assistant", ...}`, `{"type": "user", ...}` 等）
- 可通过 `tail -f <worktree>/.agent-log` 监控 agent 进展

- 任务目录下的 `implement.jsonl` / `check.jsonl` / `debug.jsonl`：按阶段的上下文注入记录（由 hooks/inject-subagent-context.py 读取）

---

## 配置管理

### 配置来源

按优先级从高到低（运行时）:

1. **task.json** (`.harness-cli/tasks/<task-id>/task.json`)
   - 任务级配置，权威来源
   - 加载函数: `.harness-cli/scripts/common/io.py::read_json()`

2. **worktree.yaml** (`.harness-cli/worktree.yaml`)
   - 并行相关：worktree 存储目录、复制文件列表、post_create hooks、verify 命令
   - 加载函数: `.harness-cli/scripts/common/worktree.py::load_worktree_config()`
   - 实现：手写 YAML 解析器（简化版，支持有限语法）

3. **config.yaml** (`.harness-cli/config.yaml`)
   - 项目级：session_commit_message、max_journal_lines、hooks、packages（monorepo）
   - 加载函数: `.harness-cli/scripts/common/config.py::_load_config()`
   - 提供的 getter:
     - `get_packages(repo_root)` → dict 或 None
     - `get_default_package(repo_root)` → str 或 None
     - `get_submodule_packages(repo_root)` → dict[str, str]
     - `validate_package(pkg, repo_root)` → bool

4. **.developer** (`.harness-cli/.developer`)
   - 开发者身份，格式 `name=<value>`
   - 加载函数: `.harness-cli/scripts/common/paths.py::get_developer()`

5. **环境变量**（仅 Rust 侧 init 时使用）
   - `https_proxy` / `http_proxy` / `all_proxy` —— 由 start.py 传递给子 agent (start.py:478-486)
   - `CLAUDECODE` —— 嵌套检测，start.py 会 `env.pop("CLAUDECODE")` 以允许新 Claude Code 进程启动 (start.py:488-490)

### 配置加载失败的降级

几乎所有 Python 配置加载函数在失败时返回 `{}` 或 `None`，不抛异常：

```python
# config.py 示例
def _load_config(repo_root):
    try:
        return parse_simple_yaml(content)
    except (OSError, IOError):
        return {}
```

设计哲学：**可用性 > 正确性**，让 agent 能启动比严格校验更重要

---

## 共享工具函数

### Rust 侧 (src/utils/)

| 工具 | 文件位置 | 用途 |
|------|----------|------|
| compare_versions | src/utils/compare_versions.rs | 语义版本比较（VERSION_A > VERSION_B?） |
| file_writer | src/utils/file_writer.rs | WriteMode 枚举（Ask/Force/Skip/Append）+ 写文件时的冲突处理 |
| project_detector | src/utils/project_detector.rs | `detect_project_type()` + `detect_packages()`，识别 Frontend/Backend/Fullstack 和 monorepo 包结构 |
| proxy | src/utils/proxy.rs | HTTP 代理配置（推测） |
| template_fetcher | src/utils/template_fetcher.rs | 从 `marketplace/index.json` 拉取远程模板；`parse_registry_source()` 解析 `gh:user/repo/path` 格式 |
| template_hash | src/utils/template_hash.rs | SHA256 计算 + 用户修改追踪 |

### Python 侧 (.harness-cli/scripts/common/)

| 工具 | 文件位置 | 用途 |
|------|----------|------|
| paths | common/paths.py | 路径常量（`DIR_WORKFLOW`, `FILE_TASK_JSON`）+ `get_repo_root()` / `get_main_repo_root()` / `get_developer()` / `get_tasks_dir()` 等 |
| io | common/io.py | `read_json()` / `write_json()`（UTF-8 + indent=2） |
| git | common/git.py | `run_git(args, cwd=None)` 包装 `subprocess.run(['git', ...])` |
| git_context | common/git_context.py | 获取 branch、commit、diff 等 git 上下文 |
| log | common/log.py | Colors 类 + `log_info/success/warn/error` |
| config | common/config.py | config.yaml 加载 + monorepo 包相关 getter |
| worktree | common/worktree.py | worktree.yaml 加载 + 简单 YAML 解析器 |
| cli_adapter | common/cli_adapter.py | 平台适配（见下） |
| registry | common/registry.py | registry.json 的 CRUD（`registry_add_agent`, `registry_remove_agent`, `registry_list_agents`） |
| task_store | common/task_store.py | 任务 CRUD：`cmd_create`, `cmd_archive`, `cmd_set_branch`, `cmd_set_scope`, `cmd_mark_kb`, `cmd_add_subtask`, `cmd_remove_subtask` |
| task.py (CLI 入口) | scripts/task.py | `cmd_start` / `cmd_finish` / `cmd_list` / `cmd_status` / `cmd_list_archive` / `cmd_create_pr` / `_auto_record_session` 位于此 —— 与 `task_store` 的划分：`task_store` 管数据写入，`task.py` 管生命周期编排 |
| task_utils | common/task_utils.py | 任务查找辅助（`find_task_by_name`, `resolve_task_dir`）；生命周期钩子（`run_task_hooks`）；**`refresh_global_workspace_index(repo_root)`** —— 全局 workspace/index.md 刷新的统一入口，供 `finish` / `archive` 调用，包含非阻塞 try/except（catches Exception + SystemExit） |
| add_session | scripts/add_session.py (非 common/) | Journal + 个人 index 写入；`add_session_from_task(task_json, auto_commit=False)` 供 `_auto_record_session` 调用；`main()` 处理 CLI 手动模式（默认 `auto_commit=True`） |
| update_workspace_index | scripts/update_workspace_index.py (非 common/) | 全局 Active Developers 表刷新；状态按 `_STATUS_LIFECYCLE_ORDER`（planning→in_progress→review→completed）排列，未知状态字母序追加 |
| phase | common/phase.py | 阶段管理（`get_phase_for_action`, `advance_phase`） |
| types | common/types.py | TaskData TypedDict + TaskInfo dataclass |
| developer | common/developer.py | 开发者身份管理（读写 `.developer`） |
| session_context | common/session_context.py | 构造会话上下文给 AI agent |
| task_context | common/task_context.py | 构造任务上下文 |
| packages_context | common/packages_context.py | monorepo 包上下文 |
| tasks | common/tasks.py | 任务列表查询 |
| task_queue | common/task_queue.py | 任务队列 |

### 共享路径常量 (paths.py:24-36)

```python
DIR_WORKFLOW = ".harness-cli"
DIR_WORKSPACE = "workspace"
DIR_TASKS = "tasks"
DIR_ARCHIVE = "archive"
DIR_SPEC = "spec"
DIR_SCRIPTS = "scripts"

FILE_DEVELOPER = ".developer"
FILE_CURRENT_TASK = ".current-task"
FILE_TASK_JSON = "task.json"
FILE_JOURNAL_PREFIX = "journal-"
```

修改目录名只需改这里，无需全项目搜索替换

### 关键函数 `get_repo_root()` 和 `get_main_repo_root()` (paths.py:43-97)

```python
def get_repo_root(start_path=None):
    """向上走，找到最近的包含 .harness-cli/ 的目录"""
    # ...

def get_main_repo_root(repo_root=None):
    """如果 repo_root 是 worktree，解析 .git 文件找到主仓库 root
    
    机制：worktree 的 .git 是文件（不是目录），内容形如
        gitdir: /path/to/main/.git/worktrees/<name>
    通过 .parent.parent 就能拿到主仓库 root
    """
    # ... (2026-04-11 新增)
```

第二个函数是为了修复 worktree 场景下 task.json 同步 bug 而新增（详见 decisions.md ADR-006）

---

## 多平台适配

### Rust 侧：configurators + placeholder 解析

**入口** (src/configurators/mod.rs:100-116):
```rust
pub fn configure_platform(platform: AITool, cwd: &Path) -> Result<()> {
    match platform {
        AITool::ClaudeCode => claude::configure(cwd),
        AITool::Cursor => cursor::configure(cwd),
        // ... 13 arms
    }
}
```

**Placeholder 解析** (src/configurators/shared.rs:14-19):
```rust
pub fn resolve_placeholders(content: &str) -> String {
    content.replace("{{PYTHON_CMD}}", get_python_command())
}

fn get_python_command() -> &'static str {
    if cfg!(windows) { "python" } else { "python3" }
}
```

每个 configurator 通过 `CopyOptions { placeholder_filename: Some("settings.json"), ... }` 决定是否启用解析

### Python 侧：cli_adapter

**文件**: `.harness-cli/scripts/common/cli_adapter.py`

**抽象目标**: 让 multi_agent 脚本不关心自己跑在哪个平台下

**核心 API**:
- `get_cli_adapter(platform: str) -> CLIAdapter` 工厂函数
- `CLIAdapter` 属性：
  - `cli_name`：如 "claude"
  - `requires_agent_definition_file`：是否需要 `.claude/agents/` 下的 md 文件
  - `supports_session_id_on_create`：是否支持启动时指定 session_id（Claude 支持，OpenCode 不支持）
  - `get_agent_path(agent_name, project_root)`
  - `get_non_interactive_env()`：环境变量修改
  - `build_run_command(agent, prompt, session_id, ...)`：构造 CLI 启动命令
  - `extract_session_id_from_log(log_content)`：从日志反推 session_id（OpenCode 用）
  - `get_resume_command_str(session_id, cwd)`：生成 resume 命令字符串

**平台特异性例子** (start.py:468-544):
```python
if adapter.supports_session_id_on_create:
    session_id = str(uuid.uuid4()).lower()
    session_id_file.write_text(session_id)
else:
    session_id = None  # OpenCode 启动后从日志提取
    # 启动后 poll 日志提取 session_id
```

---

## 中间件 / 拦截器 (Claude Code hooks)

### Hook 定义

文件: `embedded/templates/claude/settings.json`，部署到 `.claude/settings.json`

```json
{
  "statusLine": {
    "type": "command",
    "command": "{{PYTHON_CMD}} .claude/hooks/statusline.py"
  },
  "hooks": {
    "SessionStart": [
      {
        "matcher": "startup|clear|compact",
        "hooks": [{
          "type": "command",
          "command": "{{PYTHON_CMD}} .claude/hooks/session-start.py",
          "timeout": 10
        }]
      }
    ],
    "PreToolUse": [
      {
        "matcher": "Task|Agent",
        "hooks": [{
          "type": "command",
          "command": "{{PYTHON_CMD}} .claude/hooks/inject-subagent-context.py",
          "timeout": 30
        }]
      }
    ],
    "SubagentStop": [
      {
        "matcher": "check",
        "hooks": [{
          "type": "command",
          "command": "{{PYTHON_CMD}} .claude/hooks/ralph-loop.py",
          "timeout": 10
        }]
      }
    ]
  }
}
```

### Hook 职责

| Hook | 触发时机 | 脚本 | 超时 | 作用 |
|------|---------|------|------|------|
| statusLine | Claude Code UI 渲染状态栏 | statusline.py | - | 显示当前任务 / 分支 / 阶段 |
| SessionStart | 会话启动、清空、压缩 | session-start.py | 10s | 注入会话上下文（current-task / developer / git 状态） |
| PreToolUse | 触发 Task/Agent 工具前 | inject-subagent-context.py | 30s | 按子 agent 类型注入 jsonl 上下文（implement.jsonl / check.jsonl / debug.jsonl） |
| SubagentStop | 子 agent 停止后（matcher="check"） | ralph-loop.py | 10s | 验证输出、执行 worktree.yaml 中的 verify 命令、更新状态 |

### Hook 通信协议

- 输入：Claude Code 通过 stdin 传递 JSON 事件给 hook 脚本
- 输出：hook 脚本 stdout 的内容作为额外系统提示词注入到 Claude 上下文
- 退出码：非 0 会被视为 hook 失败，但不阻塞 Claude（通常只在日志显示）

### Ralph Loop 机制

- `hooks/ralph-loop.py` 实现了一种"检验-重试"循环
- 触发时机：某个 subagent（如 check 类型）停止后
- 行为：读取 `.harness-cli/worktree.yaml` 的 `verify` 数组，依次执行每个命令（如 `pnpm lint`, `pnpm typecheck`）
- 如果验证失败，hook 会返回一个指令让主 agent 继续修复，形成"修复→验证→再修复"的循环
- 超时 10 秒，避免无限循环

### statusLine 脚本

- 文件: `.claude/hooks/statusline.py`
- 作用: 每次 Claude Code 渲染 UI 时调用，输出一行 markdown 显示在状态栏
- 通常显示: 当前任务 ID、分支、current_phase、session 时长

---

## 共享约定总结

1. **路径**：一律通过 `common/paths.py` 的常量和 getter 获取，禁止硬编码 `.harness-cli`
2. **JSON**：一律通过 `common/io.py` 的 `read_json / write_json`（UTF-8 + indent=2）
3. **Git**：一律通过 `common/git.py::run_git`，支持 `cwd=` 在主仓和 worktree 间切换
4. **日志**：一律使用 `common/log.py` 的四个函数，避免 `print(...)` 裸调用
5. **错误降级**：配置加载失败返回空 dict，任务字段缺失返回 None，避免单点故障
6. **Placeholder**：模板中的 `{{PYTHON_CMD}}` 只在 init 阶段被 Rust 层替换；运行时不再做字符串模板
7. **平台差异**：Rust 侧通过 configurator 隔离，Python 侧通过 cli_adapter 抽象
