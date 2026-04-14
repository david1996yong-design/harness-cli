#!/usr/bin/env python3
"""
Task CRUD operations.

Provides:
    ensure_tasks_dir   - Ensure tasks directory exists
    cmd_create         - Create a new task
    cmd_archive        - Archive completed task
    cmd_set_branch     - Set git branch for task
    cmd_set_base_branch - Set PR target branch
    cmd_set_scope      - Set scope for PR title
    cmd_set_priority   - Set task priority
    cmd_add_subtask    - Link child task to parent
    cmd_remove_subtask - Unlink child task from parent
"""

from __future__ import annotations

import argparse
import re
import sys
from datetime import datetime
from pathlib import Path

from .config import (
    get_packages,
    is_monorepo,
    resolve_package,
    validate_package,
)
from .git import run_git
from .io import read_json, write_json
from .log import Colors, colored
from .paths import (
    DIR_ARCHIVE,
    DIR_TASKS,
    DIR_WORKFLOW,
    FILE_TASK_JSON,
    clear_current_task,
    generate_task_date_prefix,
    get_current_task,
    get_developer,
    get_repo_root,
    get_tasks_dir,
)
from .task_utils import (
    archive_task_complete,
    find_task_by_name,
    refresh_global_workspace_index,
    resolve_task_dir,
    run_task_hooks,
)


# =============================================================================
# Helper Functions
# =============================================================================

def _slugify(title: str) -> str:
    """Convert title to slug (only works with ASCII)."""
    result = title.lower()
    result = re.sub(r"[^a-z0-9]", "-", result)
    result = re.sub(r"-+", "-", result)
    result = result.strip("-")
    return result


def ensure_tasks_dir(repo_root: Path) -> Path:
    """Ensure tasks directory exists."""
    tasks_dir = get_tasks_dir(repo_root)
    archive_dir = tasks_dir / "archive"

    if not tasks_dir.exists():
        tasks_dir.mkdir(parents=True)
        print(colored(f"Created tasks directory: {tasks_dir}", Colors.GREEN), file=sys.stderr)

    if not archive_dir.exists():
        archive_dir.mkdir(parents=True)

    return tasks_dir


def _read_kb_section(repo_root: Path) -> str:
    """Read kb/prd/index.md and extract module table for PRD reference."""
    kb_index = repo_root / DIR_WORKFLOW / "kb" / "prd" / "index.md"
    if not kb_index.is_file():
        return ""

    try:
        content = kb_index.read_text(encoding="utf-8")
    except OSError:
        return ""

    # Extract the table under "## 文档索引"
    lines = content.splitlines()
    in_table = False
    table_lines: list[str] = []
    for line in lines:
        if line.strip().startswith("## 文档索引"):
            in_table = True
            continue
        if in_table:
            # Stop at next heading or end of file
            if line.strip().startswith("## ") or line.strip().startswith("<!-- "):
                break
            if line.strip():
                table_lines.append(line)

    if not table_lines:
        return ""

    return "\n".join(table_lines)


def _generate_prd(
    title: str,
    description: str,
    repo_root: Path,
    source_prd: str | None = None,
) -> str:
    """Generate PRD content with 7 core sections.

    Args:
        title: Task title.
        description: Task description (fills Goal section).
        repo_root: Repository root for reading kb/prd/index.md.
        source_prd: If provided, use this content directly instead of template.

    Returns:
        PRD markdown content.
    """
    if source_prd:
        return source_prd

    # Build Goal section content
    goal_content = description if description else "（说明为什么做 + 做什么）"

    # Build kb reference section
    kb_table = _read_kb_section(repo_root)
    if kb_table:
        kb_section = f"""## 相关模块参考

以下为项目已有的业务模块（来自 `kb/prd/index.md`），请标注本任务涉及的模块：

{kb_table}
"""
    else:
        kb_section = """## 相关模块参考

（项目暂无 `kb/prd/index.md`，可运行 `/hc:scan-kb` 生成业务知识库）
"""

    return f"""# {title}

## Goal

{goal_content}

## Requirements

- [ ] （请填写具体需求）

## Acceptance Criteria

- [ ] （请填写验收条件）

## Out of Scope

（明确列出本任务不做的事情，防止范围蔓延）

## Definition of Done

- [ ] 测试已添加或更新
- [ ] Lint / 类型检查通过
- [ ] 如行为变更，文档已更新

## 测试方案

> 框架无关：根据本项目实际语言/技术栈选择测试框架，本章节用于规划"测什么 / 怎么覆盖"。

### 1. 测试范围
- （本次变更涉及的模块/接口/页面）

### 2. 测试框架选型
- 主测试框架：（pytest / cargo test / JUnit / GoogleTest / go test / ctest / Jest / ... 根据本项目语言选择）
- 执行命令：（例：`pytest tests/`、`cargo test --test xxx`、`mvn test -pl module`）

### 3. 测试类型矩阵

| 类型 | 是否需要 | 说明 |
|------|---------|------|
| 单元测试 | ☐ | |
| 集成测试 | ☐ | |
| 手动测试 | ☐ | |
| 性能/压力 | ☐ | |

### 4. 关键用例
- 正常路径：
- 异常路径：
- 边界条件：

### 5. 回归影响面 & 风险
- 影响面：
- 风险与未覆盖：

## Technical Notes

（相关文件路径、技术约束、参考链接）

{kb_section}"""


TEST_REPORT_TEMPLATE = """# 测试报告

> ⚠ 提交 PR 前请填写本文件，否则 PR body 将带「⚠ 测试报告未填写」标记。
> 框架无关：运行命令、覆盖率工具等请按本项目实际栈填写。

## 1. 执行环境
- 分支 / commit：
- 运行命令：（与 PRD 中「测试框架选型」对应，例：`pytest -v`、`cargo test`、`mvn test`）
- 环境（OS / 依赖版本）：

## 2. 用例执行结果

| # | 用例 | 类型 | 结果 | 备注 |
|---|------|------|------|------|
| 1 | | unit / integration / manual | ✅ / ❌ | |

## 3. 覆盖率 / 性能数据
- （根据所用框架填入：pytest --cov、cargo tarpaulin、JaCoCo、benchmark 等）

## 4. 已知问题与跳过项
- （未通过用例、跳过原因）

## 5. 回归验证
- （影响面回归结论）
"""


def _generate_test_report(task_dir: Path) -> None:
    """Auto-generate test-report.md template alongside prd.md.

    Framework-agnostic: developers fill in actual commands/results based on
    the project's language and chosen test framework.
    """
    report_path = task_dir / "test-report.md"
    if report_path.exists():
        return
    report_path.write_text(TEST_REPORT_TEMPLATE, encoding="utf-8")


def is_test_report_filled(report_path: Path) -> bool:
    """Return True if test-report.md exists, is non-empty, and differs from the
    initial template (i.e. developer has actually written content)."""
    if not report_path.is_file():
        return False
    try:
        content = report_path.read_text(encoding="utf-8")
    except Exception:
        return False
    if not content.strip():
        return False
    return content.strip() != TEST_REPORT_TEMPLATE.strip()


# =============================================================================
# Command: create
# =============================================================================

def cmd_create(args: argparse.Namespace) -> int:
    """Create a new task."""
    repo_root = get_repo_root()

    if not args.title:
        print(colored("Error: title is required", Colors.RED), file=sys.stderr)
        return 1

    # Validate --package (CLI source: fail-fast)
    package: str | None = getattr(args, "package", None)
    if not is_monorepo(repo_root):
        # Single-repo: ignore --package, no package prefix
        if package:
            print(colored(f"Warning: --package ignored in single-repo project", Colors.YELLOW), file=sys.stderr)
        package = None
    elif package:
        if not validate_package(package, repo_root):
            packages = get_packages(repo_root)
            available = ", ".join(sorted(packages.keys())) if packages else "(none)"
            print(colored(f"Error: unknown package '{package}'. Available: {available}", Colors.RED), file=sys.stderr)
            return 1
    else:
        # Inferred: default_package → None (no task.json yet for create)
        package = resolve_package(repo_root=repo_root)

    # Default assignee to current developer
    assignee = args.assignee
    if not assignee:
        assignee = get_developer(repo_root)
        if not assignee:
            print(colored("Error: No developer set. Run init_developer.py first or use --assignee", Colors.RED), file=sys.stderr)
            return 1

    ensure_tasks_dir(repo_root)

    # Get current developer as creator
    creator = get_developer(repo_root) or assignee

    # Generate slug if not provided
    slug = args.slug or _slugify(args.title)
    if not slug:
        print(colored("Error: could not generate slug from title", Colors.RED), file=sys.stderr)
        return 1

    # Create task directory with MM-DD-slug format
    tasks_dir = get_tasks_dir(repo_root)
    date_prefix = generate_task_date_prefix()
    dir_name = f"{date_prefix}-{slug}"
    task_dir = tasks_dir / dir_name
    task_json_path = task_dir / FILE_TASK_JSON

    if task_dir.exists():
        print(colored(f"Warning: Task directory already exists: {dir_name}", Colors.YELLOW), file=sys.stderr)
    else:
        task_dir.mkdir(parents=True)

    today = datetime.now().strftime("%Y-%m-%d")

    # Record current branch as base_branch (PR target)
    _, branch_out, _ = run_git(["branch", "--show-current"], cwd=repo_root)
    current_branch = branch_out.strip() or "main"

    task_data = {
        "id": slug,
        "name": slug,
        "title": args.title,
        "description": args.description or "",
        "status": "planning",
        "dev_type": None,
        "scope": None,
        "package": package,
        "priority": args.priority,
        "creator": creator,
        "assignee": assignee,
        "createdAt": today,
        "completedAt": None,
        "branch": None,
        "base_branch": current_branch,
        "worktree_path": None,
        "current_phase": 0,
        "next_action": [
            {"phase": 1, "action": "implement"},
            {"phase": 2, "action": "check"},
            {"phase": 3, "action": "finish"},
            {"phase": 4, "action": "create-pr"},
        ],
        "commit": None,
        "pr_url": None,
        "kb_status": "needed",
        "subtasks": [],
        "children": [],
        "parent": None,
        "relatedFiles": [],
        "notes": "",
        "meta": {},
    }

    write_json(task_json_path, task_data)

    # Handle --parent: establish bidirectional link
    if args.parent:
        parent_dir = resolve_task_dir(args.parent, repo_root)
        parent_json_path = parent_dir / FILE_TASK_JSON
        if not parent_json_path.is_file():
            print(colored(f"Warning: Parent task.json not found: {args.parent}", Colors.YELLOW), file=sys.stderr)
        else:
            parent_data = read_json(parent_json_path)
            if parent_data:
                # Add child to parent's children list
                parent_children = parent_data.get("children", [])
                if dir_name not in parent_children:
                    parent_children.append(dir_name)
                    parent_data["children"] = parent_children
                    write_json(parent_json_path, parent_data)

                # Set parent in child's task.json
                task_data["parent"] = parent_dir.name
                write_json(task_json_path, task_data)

                print(colored(f"Linked as child of: {parent_dir.name}", Colors.GREEN), file=sys.stderr)

    # Generate prd.md (only if it doesn't already exist)
    prd_path = task_dir / "prd.md"
    if not prd_path.is_file():
        prd_content = _generate_prd(
            title=args.title,
            description=args.description or "",
            repo_root=repo_root,
        )
        prd_path.write_text(prd_content, encoding="utf-8")

    # Generate test-report.md template (framework-agnostic)
    _generate_test_report(task_dir)

    print(colored(f"Created task: {dir_name}", Colors.GREEN), file=sys.stderr)
    print("", file=sys.stderr)
    print(colored("Next steps:", Colors.BLUE), file=sys.stderr)
    print("  1. Fill in prd.md with detailed requirements", file=sys.stderr)
    print("  2. Run: python3 task.py init-context <dir> <dev_type>", file=sys.stderr)
    print("  3. Run: python3 task.py start <dir>", file=sys.stderr)
    print("", file=sys.stderr)

    # Output relative path for script chaining
    print(f"{DIR_WORKFLOW}/{DIR_TASKS}/{dir_name}")

    run_task_hooks("after_create", task_json_path, repo_root)
    return 0


# =============================================================================
# Command: archive
# =============================================================================

def cmd_archive(args: argparse.Namespace) -> int:
    """Archive completed task."""
    repo_root = get_repo_root()
    task_name = args.name

    if not task_name:
        print(colored("Error: Task name is required", Colors.RED), file=sys.stderr)
        return 1

    tasks_dir = get_tasks_dir(repo_root)

    # Find task directory
    task_dir = find_task_by_name(task_name, tasks_dir)

    if not task_dir or not task_dir.is_dir():
        print(colored(f"Error: Task not found: {task_name}", Colors.RED), file=sys.stderr)
        print("Active tasks:", file=sys.stderr)
        # Import lazily to avoid circular dependency
        from .tasks import iter_active_tasks
        for t in iter_active_tasks(tasks_dir):
            print(f"  - {t.dir_name}/", file=sys.stderr)
        return 1

    dir_name = task_dir.name
    task_json_path = task_dir / FILE_TASK_JSON

    # KB status gate — block archive if task hasn't declared KB outcome.
    # The AI (during finish/scan-kb) is responsible for flipping this field
    # to "updated" or "not_required" using `task.py mark-kb`.
    if task_json_path.is_file():
        pre_data = read_json(task_json_path)
        if pre_data:
            kb_status = pre_data.get("kb_status", "needed")
            if kb_status == "needed":
                print(
                    colored(
                        f"Error: cannot archive '{dir_name}' — kb_status is 'needed'",
                        Colors.RED,
                    ),
                    file=sys.stderr,
                )
                print("  KB must be resolved before archive. Choose one:", file=sys.stderr)
                print(
                    "    • If this task changed business logic: run /hc:scan-kb to "
                    "refresh kb/prd/, then retry archive",
                    file=sys.stderr,
                )
                print(
                    f"    • If this task does not affect KB: run\n"
                    f"      python3 .harness-cli/scripts/task.py mark-kb not-required {dir_name}\n"
                    f"      and retry archive",
                    file=sys.stderr,
                )
                return 1

    # Update status before archiving
    today = datetime.now().strftime("%Y-%m-%d")
    if task_json_path.is_file():
        data = read_json(task_json_path)
        if data:
            data["status"] = "completed"
            data["completedAt"] = today
            write_json(task_json_path, data)

            # Handle subtask relationships on archive
            task_parent = data.get("parent")
            task_children = data.get("children", [])

            # If this is a child, remove from parent's children list
            if task_parent:
                parent_dir = find_task_by_name(task_parent, tasks_dir)
                if parent_dir:
                    parent_json = parent_dir / FILE_TASK_JSON
                    if parent_json.is_file():
                        parent_data = read_json(parent_json)
                        if parent_data:
                            parent_children = parent_data.get("children", [])
                            if dir_name in parent_children:
                                parent_children.remove(dir_name)
                                parent_data["children"] = parent_children
                                write_json(parent_json, parent_data)

            # If this is a parent, clear parent field in all children
            if task_children:
                for child_name in task_children:
                    child_dir_path = find_task_by_name(child_name, tasks_dir)
                    if child_dir_path:
                        child_json = child_dir_path / FILE_TASK_JSON
                        if child_json.is_file():
                            child_data = read_json(child_json)
                            if child_data:
                                child_data["parent"] = None
                                write_json(child_json, child_data)

    # Clear if current task
    current = get_current_task(repo_root)
    if current and dir_name in current:
        clear_current_task(repo_root)

    # Archive
    result = archive_task_complete(task_dir, repo_root)
    if "archived_to" in result:
        archive_dest = Path(result["archived_to"])
        year_month = archive_dest.parent.name
        print(colored(f"Archived: {dir_name} -> archive/{year_month}/", Colors.GREEN), file=sys.stderr)

        # Auto-commit unless --no-commit
        if not getattr(args, "no_commit", False):
            _auto_commit_archive(dir_name, repo_root)

        # Return the archive path
        print(f"{DIR_WORKFLOW}/{DIR_TASKS}/{DIR_ARCHIVE}/{year_month}/{dir_name}")

        # Run hooks with the archived path
        archived_json = archive_dest / FILE_TASK_JSON
        run_task_hooks("after_archive", archived_json, repo_root)

        # Refresh global workspace index — archived task should no longer appear
        # in the Active Developers table
        refresh_global_workspace_index(repo_root)
        return 0

    return 1


def _auto_commit_archive(task_name: str, repo_root: Path) -> None:
    """Stage .harness-cli/tasks/ changes and commit after archive."""
    tasks_rel = f"{DIR_WORKFLOW}/{DIR_TASKS}"
    run_git(["add", "-A", tasks_rel], cwd=repo_root)

    # Check if there are staged changes
    rc, _, _ = run_git(
        ["diff", "--cached", "--quiet", "--", tasks_rel], cwd=repo_root
    )
    if rc == 0:
        print("[OK] No task changes to commit.", file=sys.stderr)
        return

    commit_msg = f"chore(task): archive {task_name}"
    rc, _, err = run_git(["commit", "-m", commit_msg], cwd=repo_root)
    if rc == 0:
        print(f"[OK] Auto-committed: {commit_msg}", file=sys.stderr)
    else:
        print(f"[WARN] Auto-commit failed: {err.strip()}", file=sys.stderr)


# =============================================================================
# Command: add-subtask
# =============================================================================

def cmd_add_subtask(args: argparse.Namespace) -> int:
    """Link a child task to a parent task."""
    repo_root = get_repo_root()

    parent_dir = resolve_task_dir(args.parent_dir, repo_root)
    child_dir = resolve_task_dir(args.child_dir, repo_root)

    parent_json_path = parent_dir / FILE_TASK_JSON
    child_json_path = child_dir / FILE_TASK_JSON

    if not parent_json_path.is_file():
        print(colored(f"Error: Parent task.json not found: {args.parent_dir}", Colors.RED), file=sys.stderr)
        return 1

    if not child_json_path.is_file():
        print(colored(f"Error: Child task.json not found: {args.child_dir}", Colors.RED), file=sys.stderr)
        return 1

    parent_data = read_json(parent_json_path)
    child_data = read_json(child_json_path)

    if not parent_data or not child_data:
        print(colored("Error: Failed to read task.json", Colors.RED), file=sys.stderr)
        return 1

    # Check if child already has a parent
    existing_parent = child_data.get("parent")
    if existing_parent:
        print(colored(f"Error: Child task already has a parent: {existing_parent}", Colors.RED), file=sys.stderr)
        return 1

    # Add child to parent's children list
    parent_children = parent_data.get("children", [])
    child_dir_name = child_dir.name
    if child_dir_name not in parent_children:
        parent_children.append(child_dir_name)
        parent_data["children"] = parent_children

    # Set parent in child's task.json
    child_data["parent"] = parent_dir.name

    # Write both
    write_json(parent_json_path, parent_data)
    write_json(child_json_path, child_data)

    print(colored(f"Linked: {child_dir.name} -> {parent_dir.name}", Colors.GREEN), file=sys.stderr)
    return 0


# =============================================================================
# Command: remove-subtask
# =============================================================================

def cmd_remove_subtask(args: argparse.Namespace) -> int:
    """Unlink a child task from a parent task."""
    repo_root = get_repo_root()

    parent_dir = resolve_task_dir(args.parent_dir, repo_root)
    child_dir = resolve_task_dir(args.child_dir, repo_root)

    parent_json_path = parent_dir / FILE_TASK_JSON
    child_json_path = child_dir / FILE_TASK_JSON

    if not parent_json_path.is_file():
        print(colored(f"Error: Parent task.json not found: {args.parent_dir}", Colors.RED), file=sys.stderr)
        return 1

    if not child_json_path.is_file():
        print(colored(f"Error: Child task.json not found: {args.child_dir}", Colors.RED), file=sys.stderr)
        return 1

    parent_data = read_json(parent_json_path)
    child_data = read_json(child_json_path)

    if not parent_data or not child_data:
        print(colored("Error: Failed to read task.json", Colors.RED), file=sys.stderr)
        return 1

    # Remove child from parent's children list
    parent_children = parent_data.get("children", [])
    child_dir_name = child_dir.name
    if child_dir_name in parent_children:
        parent_children.remove(child_dir_name)
        parent_data["children"] = parent_children

    # Clear parent in child's task.json
    child_data["parent"] = None

    # Write both
    write_json(parent_json_path, parent_data)
    write_json(child_json_path, child_data)

    print(colored(f"Unlinked: {child_dir.name} from {parent_dir.name}", Colors.GREEN), file=sys.stderr)
    return 0


# =============================================================================
# Command: set-branch
# =============================================================================

def cmd_set_branch(args: argparse.Namespace) -> int:
    """Set git branch for task."""
    repo_root = get_repo_root()
    target_dir = resolve_task_dir(args.dir, repo_root)
    branch = args.branch

    if not branch:
        print(colored("Error: Missing arguments", Colors.RED))
        print("Usage: python3 task.py set-branch <task-dir> <branch-name>")
        return 1

    task_json = target_dir / FILE_TASK_JSON
    if not task_json.is_file():
        print(colored(f"Error: task.json not found at {target_dir}", Colors.RED))
        return 1

    data = read_json(task_json)
    if not data:
        return 1

    data["branch"] = branch
    write_json(task_json, data)

    print(colored(f"✓ Branch set to: {branch}", Colors.GREEN))
    print()
    print(colored("Now you can start the multi-agent pipeline:", Colors.BLUE))
    print(f"  python3 ./.harness-cli/scripts/multi_agent/start.py {args.dir}")
    return 0


# =============================================================================
# Command: set-base-branch
# =============================================================================

def cmd_set_base_branch(args: argparse.Namespace) -> int:
    """Set the base branch (PR target) for task."""
    repo_root = get_repo_root()
    target_dir = resolve_task_dir(args.dir, repo_root)
    base_branch = args.base_branch

    if not base_branch:
        print(colored("Error: Missing arguments", Colors.RED))
        print("Usage: python3 task.py set-base-branch <task-dir> <base-branch>")
        print("Example: python3 task.py set-base-branch <dir> develop")
        print()
        print("This sets the target branch for PR (the branch your feature will merge into).")
        return 1

    task_json = target_dir / FILE_TASK_JSON
    if not task_json.is_file():
        print(colored(f"Error: task.json not found at {target_dir}", Colors.RED))
        return 1

    data = read_json(task_json)
    if not data:
        return 1

    data["base_branch"] = base_branch
    write_json(task_json, data)

    print(colored(f"✓ Base branch set to: {base_branch}", Colors.GREEN))
    print(f"  PR will target: {base_branch}")
    return 0


# =============================================================================
# Command: set-scope
# =============================================================================

def cmd_set_scope(args: argparse.Namespace) -> int:
    """Set scope for PR title."""
    repo_root = get_repo_root()
    target_dir = resolve_task_dir(args.dir, repo_root)
    scope = args.scope

    if not scope:
        print(colored("Error: Missing arguments", Colors.RED))
        print("Usage: python3 task.py set-scope <task-dir> <scope>")
        return 1

    task_json = target_dir / FILE_TASK_JSON
    if not task_json.is_file():
        print(colored(f"Error: task.json not found at {target_dir}", Colors.RED))
        return 1

    data = read_json(task_json)
    if not data:
        return 1

    data["scope"] = scope
    write_json(task_json, data)

    print(colored(f"✓ Scope set to: {scope}", Colors.GREEN))
    return 0


# =============================================================================
# Command: mark-kb
# =============================================================================

_KB_STATUS_VALUES = {"needed", "updated", "not_required"}


def cmd_mark_kb(args: argparse.Namespace) -> int:
    """Set kb_status field on task.json.

    Acceptable statuses:
      - needed        : KB check still outstanding (default for new tasks)
      - updated       : KB has been refreshed to reflect this task's changes
      - not-required  : task doesn't affect KB (docs-only, typo fix, test tweak, etc.)

    Archive is blocked until kb_status leaves 'needed'.
    """
    repo_root = get_repo_root()

    # Normalize hyphenated CLI input to snake_case canonical value
    raw_status = (args.status or "").strip().replace("-", "_")
    if raw_status not in _KB_STATUS_VALUES:
        print(
            colored(
                f"Error: invalid status '{args.status}'. "
                f"Expected one of: needed | updated | not-required",
                Colors.RED,
            ),
            file=sys.stderr,
        )
        return 1

    # Resolve target task: explicit arg > current task
    task_input = getattr(args, "task", None)
    if task_input:
        target_dir = resolve_task_dir(task_input, repo_root)
    else:
        current = get_current_task(repo_root)
        if not current:
            print(
                colored(
                    "Error: no task specified and no current task set. "
                    "Pass <task-name> or run `task.py start <dir>` first.",
                    Colors.RED,
                ),
                file=sys.stderr,
            )
            return 1
        target_dir = repo_root / current

    if not target_dir.is_dir():
        print(colored(f"Error: task directory not found: {target_dir}", Colors.RED), file=sys.stderr)
        return 1

    task_json = target_dir / FILE_TASK_JSON
    if not task_json.is_file():
        print(colored(f"Error: task.json not found at {target_dir}", Colors.RED), file=sys.stderr)
        return 1

    data = read_json(task_json)
    if not data:
        return 1

    data["kb_status"] = raw_status
    write_json(task_json, data)

    print(colored(f"✓ kb_status set to: {raw_status} ({target_dir.name})", Colors.GREEN))
    return 0


# =============================================================================
# Command: set-priority
# =============================================================================

VALID_PRIORITIES = ("P0", "P1", "P2", "P3")


def cmd_set_priority(args: argparse.Namespace) -> int:
    """Set priority for task."""
    repo_root = get_repo_root()
    target_dir = resolve_task_dir(args.dir, repo_root)
    priority = args.priority

    if not priority:
        print(colored("Error: Missing arguments", Colors.RED))
        print("Usage: python3 task.py set-priority <task-dir> <P0|P1|P2|P3>")
        return 1

    if priority not in VALID_PRIORITIES:
        print(colored(f"Error: Invalid priority '{priority}'. Must be one of: {', '.join(VALID_PRIORITIES)}", Colors.RED))
        return 1

    task_json = target_dir / FILE_TASK_JSON
    if not task_json.is_file():
        print(colored(f"Error: task.json not found at {target_dir}", Colors.RED))
        return 1

    data = read_json(task_json)
    if not data:
        return 1

    data["priority"] = priority
    write_json(task_json, data)

    print(colored(f"✓ Priority set to: {priority}", Colors.GREEN))
    return 0
