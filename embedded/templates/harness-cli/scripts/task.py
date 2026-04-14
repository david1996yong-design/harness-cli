#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Task Management Script for Multi-Agent Pipeline.

Usage:
    python3 task.py create "<title>" [--slug <name>] [--assignee <dev>] [--priority P0|P1|P2|P3] [--parent <dir>] [--package <pkg>]
    python3 task.py init-context <dir> <type> [--package <pkg>]  # Initialize jsonl files
    python3 task.py add-context <dir> <file> <path> [reason] # Add jsonl entry
    python3 task.py validate <dir>              # Validate jsonl files
    python3 task.py list-context <dir>          # List jsonl entries
    python3 task.py start <dir>                 # Set as current task
    python3 task.py finish                      # Clear current task
    python3 task.py set-branch <dir> <branch>   # Set git branch
    python3 task.py set-base-branch <dir> <branch>  # Set PR target branch
    python3 task.py set-scope <dir> <scope>     # Set scope for PR title
    python3 task.py mark-kb <status> [<task>]   # Set kb_status (needed|updated|not-required)
    python3 task.py create-pr [dir] [--dry-run] # Create PR from task
    python3 task.py archive <task-name>         # Archive completed task
    python3 task.py list [--detail]             # List active tasks
    python3 task.py status [--mine] [--json]    # Task status dashboard
    python3 task.py list-archive [month]        # List archived tasks
    python3 task.py add-subtask <parent-dir> <child-dir>     # Link child to parent
    python3 task.py remove-subtask <parent-dir> <child-dir>  # Unlink child from parent
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from datetime import datetime

from common.git import run_git
from common.io import read_json, write_json
from common.log import Colors, colored
from common.paths import (
    DIR_WORKFLOW,
    DIR_TASKS,
    FILE_TASK_JSON,
    get_repo_root,
    get_developer,
    get_tasks_dir,
    get_current_task,
    set_current_task,
    clear_current_task,
)
from common.task_utils import (
    refresh_global_workspace_index,
    resolve_task_dir,
    run_task_hooks,
)
from common.tasks import iter_active_tasks, children_progress
from common.types import TaskInfo

# Import command handlers from split modules (also re-exports for plan.py compatibility)
from common.task_store import (
    cmd_create,
    cmd_archive,
    cmd_set_branch,
    cmd_set_base_branch,
    cmd_set_scope,
    cmd_mark_kb,
    cmd_add_subtask,
    cmd_remove_subtask,
)
from common.task_context import (
    cmd_init_context,
    cmd_add_context,
    cmd_validate,
    cmd_list_context,
)
from common.weekly_report import generate_weekly_report


# =============================================================================
# Command: start / finish
# =============================================================================

def cmd_start(args: argparse.Namespace) -> int:
    """Set current task."""
    repo_root = get_repo_root()
    task_input = args.dir

    if not task_input:
        print(colored("Error: task directory or name required", Colors.RED))
        return 1

    # Resolve task directory (supports task name, relative path, or absolute path)
    full_path = resolve_task_dir(task_input, repo_root)

    if not full_path.is_dir():
        print(colored(f"Error: Task not found: {task_input}", Colors.RED))
        print("Hint: Use task name (e.g., 'my-task') or full path (e.g., '.harness-cli/tasks/01-31-my-task')")
        return 1

    # Convert to relative path for storage
    try:
        task_dir = full_path.relative_to(repo_root).as_posix()
    except ValueError:
        task_dir = str(full_path)

    if set_current_task(task_dir, repo_root):
        print(colored(f"✓ Current task set to: {task_dir}", Colors.GREEN))

        task_json_path = full_path / FILE_TASK_JSON
        _promote_status_on_start(task_json_path)

        print()
        print(colored("The hook will now inject context from this task's jsonl files.", Colors.BLUE))

        run_task_hooks("after_start", task_json_path, repo_root)
        return 0
    else:
        print(colored("Error: Failed to set current task", Colors.RED))
        return 1


def _promote_status_on_start(task_json_path: Path) -> None:
    """Bump status planning → in_progress on `start` (idempotent)."""
    if not task_json_path.is_file():
        return
    data = read_json(task_json_path)
    if not isinstance(data, dict):
        return
    if data.get("status") == "planning":
        data["status"] = "in_progress"
        write_json(task_json_path, data)
        print(colored("  status: planning → in_progress", Colors.DIM))


def _finalize_task_on_finish(task_dir: Path, task_json_path: Path, repo_root: Path) -> None:
    """Write completion fields on `finish` (idempotent).

    - status → completed
    - completedAt → today (if empty)
    - commit → HEAD of worktree (or main repo, if no worktree)
    - current_phase → len(next_action)
    """
    if not task_json_path.is_file():
        return
    data = read_json(task_json_path)
    if not isinstance(data, dict):
        return

    changed = False

    if data.get("status") != "completed":
        data["status"] = "completed"
        changed = True

    if not data.get("completedAt"):
        data["completedAt"] = datetime.now().strftime("%Y-%m-%d")
        changed = True

    # Resolve commit HEAD from worktree if available, else repo_root
    if not data.get("commit"):
        git_cwd = repo_root
        wt = data.get("worktree_path")
        if isinstance(wt, str) and wt:
            wt_path = Path(wt)
            if wt_path.is_dir():
                git_cwd = wt_path
        rc, out, _ = run_git(["rev-parse", "HEAD"], cwd=git_cwd)
        if rc == 0 and out.strip():
            data["commit"] = out.strip()
            changed = True

    # Advance current_phase to terminal value
    next_action = data.get("next_action") or []
    total = len(next_action) if isinstance(next_action, list) else 0
    if total > 0 and (data.get("current_phase") or 0) < total:
        data["current_phase"] = total
        changed = True

    if changed:
        write_json(task_json_path, data)
        print(colored(
            f"  finalized: status=completed, phase={data.get('current_phase')}, commit={(data.get('commit') or '')[:8]}",
            Colors.DIM,
        ))


def _auto_record_session(task_json_path: Path, repo_root: Path) -> None:
    """Automatically record a session from the completed task.

    Called by cmd_finish. For symmetry, cmd_archive only needs to call
    refresh_global_workspace_index (the session was already recorded at finish time).

    Two independent side-effects, isolated so one failure does not mask the other:
      1. Session recording (journal + personal index.md) via add_session_from_task.
         Wrapped in try/except here (catches Exception + SystemExit from
         ensure_developer's sys.exit).
      2. Global workspace index refresh via refresh_global_workspace_index.
         The helper handles its own try/except internally.

    Non-blocking: any failure prints a warning but does not interrupt the finish flow.
    """
    # Ensure scripts dir is on path for sibling imports (add_session is not in common/)
    scripts_dir = str(Path(__file__).resolve().parent)
    if scripts_dir not in sys.path:
        sys.path.insert(0, scripts_dir)

    # Step 1: record session to journal + personal index
    try:
        from add_session import add_session_from_task

        print()
        print(colored("Recording session...", Colors.BLUE))
        rc = add_session_from_task(task_json_path, auto_commit=False)
        if rc == 0:
            print(colored("✓ Session auto-recorded to journal", Colors.GREEN))
        else:
            print(colored("⚠ Session recording skipped (non-fatal)", Colors.YELLOW))
    except (Exception, SystemExit) as e:
        print(colored(f"⚠ Session recording failed: {e}", Colors.YELLOW), file=sys.stderr)

    # Step 2: refresh global workspace/index.md (Active Developers table)
    refresh_global_workspace_index(repo_root)


def cmd_finish(args: argparse.Namespace) -> int:
    """Clear current task and finalize status fields."""
    repo_root = get_repo_root()
    current = get_current_task(repo_root)

    if not current:
        print(colored("No current task set", Colors.YELLOW))
        return 0

    # Resolve task.json path before clearing
    task_dir = repo_root / current
    task_json_path = task_dir / FILE_TASK_JSON

    _finalize_task_on_finish(task_dir, task_json_path, repo_root)

    # Auto-record session BEFORE clearing current task (task.json still accessible)
    if task_json_path.is_file():
        _auto_record_session(task_json_path, repo_root)

    clear_current_task(repo_root)
    print(colored(f"✓ Cleared current task (was: {current})", Colors.GREEN))

    if task_json_path.is_file():
        run_task_hooks("after_finish", task_json_path, repo_root)
    return 0


# =============================================================================
# Command: list
# =============================================================================

def cmd_list(args: argparse.Namespace) -> int:
    """List active tasks."""
    repo_root = get_repo_root()
    tasks_dir = get_tasks_dir(repo_root)
    current_task = get_current_task(repo_root)
    developer = get_developer(repo_root)
    filter_mine = args.mine
    filter_status = args.status
    detail = getattr(args, "detail", False)

    if filter_mine:
        if not developer:
            print(colored("Error: No developer set. Run init_developer.py first", Colors.RED), file=sys.stderr)
            return 1
        print(colored(f"My tasks (assignee: {developer}):", Colors.BLUE))
    else:
        print(colored("All active tasks:", Colors.BLUE))
    print()

    # Single pass: collect all tasks via shared iterator
    all_tasks = {t.dir_name: t for t in iter_active_tasks(tasks_dir)}
    all_statuses = {name: t.status for name, t in all_tasks.items()}

    # Display tasks hierarchically
    count = 0

    def _print_task(dir_name: str, indent: int = 0) -> None:
        nonlocal count
        t = all_tasks[dir_name]

        # Apply --mine filter
        if filter_mine and (t.assignee or "-") != developer:
            return

        # Apply --status filter
        if filter_status and t.status != filter_status:
            return

        relative_path = f"{DIR_WORKFLOW}/{DIR_TASKS}/{dir_name}"
        marker = ""
        if relative_path == current_task:
            marker = f" {colored('<- current', Colors.GREEN)}"

        # Children progress
        progress = children_progress(t.children, all_statuses)

        # Package tag
        pkg_tag = f" @{t.package}" if t.package else ""

        prefix = "  " * indent + "  - "

        if detail:
            # Detail mode: multi-line display per task
            print(f"{prefix}{colored(dir_name, Colors.CYAN)}/{marker}")
            detail_prefix = "  " * indent + "    "
            print(f"{detail_prefix}priority:  {t.priority}")
            print(f"{detail_prefix}title:     {t.title}")
            print(f"{detail_prefix}status:    {t.status}")
            print(f"{detail_prefix}assignee:  {t.assignee or '-'}")
            if t.branch:
                print(f"{detail_prefix}branch:    {t.branch}")
            created_at = t.raw.get("createdAt", "-")
            print(f"{detail_prefix}created:   {created_at}")
            if t.package:
                print(f"{detail_prefix}package:   {t.package}")
            if t.children:
                print(f"{detail_prefix}children:  {progress.strip()}")
            print()
        elif filter_mine:
            print(f"{prefix}{dir_name}/ ({t.status}){pkg_tag}{progress}{marker}")
        else:
            print(f"{prefix}{dir_name}/ ({t.status}){pkg_tag}{progress} [{colored(t.assignee or '-', Colors.CYAN)}]{marker}")
        count += 1

        # Print children indented
        for child_name in t.children:
            if child_name in all_tasks:
                _print_task(child_name, indent + 1)

    # Display only top-level tasks (those without a parent)
    for dir_name in sorted(all_tasks.keys()):
        if not all_tasks[dir_name].parent:
            _print_task(dir_name)

    if count == 0:
        if filter_mine:
            print("  (no tasks assigned to you)")
        else:
            print("  (no active tasks)")

    print()
    print(f"Total: {count} task(s)")
    return 0


# =============================================================================
# Command: status
# =============================================================================

def cmd_status(args: argparse.Namespace) -> int:
    """Task status dashboard."""
    import json as json_mod

    repo_root = get_repo_root()
    tasks_dir = get_tasks_dir(repo_root)
    developer = get_developer(repo_root)
    filter_mine = getattr(args, "mine", False)
    output_json = getattr(args, "json", False)

    if filter_mine and not developer:
        print(colored("Error: No developer set. Run init_developer.py first", Colors.RED), file=sys.stderr)
        return 1

    # Collect all tasks
    all_tasks_list = list(iter_active_tasks(tasks_dir))

    # Apply --mine filter
    if filter_mine:
        all_tasks_list = [t for t in all_tasks_list if (t.assignee or "-") == developer]

    # Group by status
    status_order = ["planning", "in_progress", "review", "completed"]
    grouped: dict[str, list] = {s: [] for s in status_order}
    other_group: list = []

    for t in all_tasks_list:
        if t.status in grouped:
            grouped[t.status].append(t)
        else:
            other_group.append(t)

    # Priority stats
    priority_counts = {"P0": 0, "P1": 0, "P2": 0, "P3": 0}
    status_counts: dict[str, int] = {}
    for t in all_tasks_list:
        if t.priority in priority_counts:
            priority_counts[t.priority] += 1
        status_counts[t.status] = status_counts.get(t.status, 0) + 1

    total = len(all_tasks_list)

    # JSON output
    if output_json:
        def _task_to_dict(t: TaskInfo) -> dict:
            return {
                "dir_name": t.dir_name,
                "priority": t.priority,
                "title": t.title,
                "status": t.status,
                "assignee": t.assignee or "-",
                "branch": t.branch,
                "package": t.package,
                "createdAt": t.raw.get("createdAt", ""),
                "children": list(t.children),
                "parent": t.parent,
            }

        tasks_list: list[dict] = []
        by_status: dict[str, list[dict]] = {}

        for status in status_order:
            by_status[status] = []
            for t in grouped[status]:
                task_dict = _task_to_dict(t)
                by_status[status].append(task_dict)
                tasks_list.append(task_dict)
        if other_group:
            by_status["other"] = []
            for t in other_group:
                task_dict = _task_to_dict(t)
                by_status["other"].append(task_dict)
                tasks_list.append(task_dict)

        data = {
            "tasks": tasks_list,
            "by_status": by_status,
            "priority_counts": priority_counts,
            "status_counts": status_counts,
            "total": total,
        }
        print(json_mod.dumps(data, indent=2, ensure_ascii=False))
        return 0

    # Human-readable output
    if filter_mine:
        print(colored(f"Task Status Dashboard (assignee: {developer})", Colors.BLUE))
    else:
        print(colored("Task Status Dashboard", Colors.BLUE))
    print(colored("=" * 60, Colors.DIM))
    print()

    for status in status_order:
        tasks_in_group = grouped[status]
        if not tasks_in_group:
            continue

        status_label = status.upper().replace("_", " ")
        print(colored(f"[{status_label}] ({len(tasks_in_group)})", Colors.YELLOW))
        print(colored("-" * 40, Colors.DIM))

        for t in tasks_in_group:
            branch_info = f"  branch: {t.branch}" if t.branch else ""
            print(f"  {colored(t.priority, Colors.RED if t.priority == 'P0' else Colors.NC)} | {t.title}")
            print(f"       assignee: {t.assignee or '-'}{branch_info}")
            if t.package:
                print(f"       package: {t.package}")
        print()

    # Other statuses
    if other_group:
        print(colored(f"[OTHER] ({len(other_group)})", Colors.YELLOW))
        print(colored("-" * 40, Colors.DIM))
        for t in other_group:
            branch_info = f"  branch: {t.branch}" if t.branch else ""
            print(f"  {t.priority} | {t.title} ({t.status})")
            print(f"       assignee: {t.assignee or '-'}{branch_info}")
        print()

    # Summary line
    print(colored("=" * 60, Colors.DIM))
    priority_str = "  ".join(f"{k}:{v}" for k, v in priority_counts.items())
    status_str = "  ".join(f"{k}:{v}" for k, v in status_counts.items())
    print(f"Total: {total} task(s)  |  Priority: {priority_str}")
    print(f"Status: {status_str}")

    return 0


# =============================================================================
# Command: list-archive
# =============================================================================

def cmd_list_archive(args: argparse.Namespace) -> int:
    """List archived tasks."""
    repo_root = get_repo_root()
    tasks_dir = get_tasks_dir(repo_root)
    archive_dir = tasks_dir / "archive"
    month = args.month

    print(colored("Archived tasks:", Colors.BLUE))
    print()

    if month:
        month_dir = archive_dir / month
        if month_dir.is_dir():
            print(f"[{month}]")
            for d in sorted(month_dir.iterdir()):
                if d.is_dir():
                    print(f"  - {d.name}/")
        else:
            print(f"  No archives for {month}")
    else:
        if archive_dir.is_dir():
            for month_dir in sorted(archive_dir.iterdir()):
                if month_dir.is_dir():
                    month_name = month_dir.name
                    count = sum(1 for d in month_dir.iterdir() if d.is_dir())
                    print(f"[{month_name}] - {count} task(s)")

    return 0


# =============================================================================
# Command: weekly-report (personal weekly aggregation)
# =============================================================================

def cmd_weekly_report(args: argparse.Namespace) -> int:
    """Generate this week's personal report and write to workspace/{dev}/reports/."""
    try:
        out_path, _ = generate_weekly_report(
            week_arg=args.week,
            dev_override=args.dev,
        )
    except Exception as e:
        print(colored(f"Error: {e}", Colors.RED))
        return 1

    try:
        rel = out_path.relative_to(get_repo_root())
    except ValueError:
        rel = out_path

    print(colored(f"✓ Weekly report written: {rel}", Colors.GREEN))
    return 0


# =============================================================================
# Command: create-pr (delegates to multi-agent script)
# =============================================================================

def cmd_create_pr(args: argparse.Namespace) -> int:
    """Create PR from task - delegates to multi_agent/create_pr.py."""
    import subprocess
    script_dir = Path(__file__).parent
    create_pr_script = script_dir / "multi_agent" / "create_pr.py"

    cmd = [sys.executable, str(create_pr_script)]
    if args.dir:
        cmd.append(args.dir)
    if args.dry_run:
        cmd.append("--dry-run")

    result = subprocess.run(cmd)
    return result.returncode


# =============================================================================
# Help
# =============================================================================

def show_usage() -> None:
    """Show usage help."""
    print("""Task Management Script for Multi-Agent Pipeline

Usage:
  python3 task.py create <title>                     Create new task directory
  python3 task.py create <title> --package <pkg>     Create task for a specific package
  python3 task.py create <title> --parent <dir>      Create task as child of parent
  python3 task.py init-context <dir> <dev_type>      Initialize jsonl files
  python3 task.py init-context <dir> <type> --package <pkg>  With explicit package
  python3 task.py add-context <dir> <jsonl> <path> [reason]  Add entry to jsonl
  python3 task.py validate <dir>                     Validate jsonl files
  python3 task.py list-context <dir>                 List jsonl entries
  python3 task.py start <dir>                        Set as current task
  python3 task.py finish                             Clear current task
  python3 task.py set-branch <dir> <branch>          Set git branch for multi-agent
  python3 task.py set-scope <dir> <scope>            Set scope for PR title
  python3 task.py mark-kb <status> [<task>]          Set kb_status (needed|updated|not-required)
  python3 task.py create-pr [dir] [--dry-run]        Create PR from task
  python3 task.py archive <task-name>                Archive completed task
  python3 task.py add-subtask <parent> <child>       Link child task to parent
  python3 task.py remove-subtask <parent> <child>    Unlink child from parent
  python3 task.py list [--mine] [--status <s>]       List tasks
  python3 task.py list --detail                      List tasks with detailed info
  python3 task.py status [--mine] [--json]           Task status dashboard
  python3 task.py list-archive [YYYY-MM]             List archived tasks
  python3 task.py weekly-report [--week YYYY-Www]    Generate personal weekly report

Arguments:
  dev_type: backend | frontend | fullstack | test | docs

Monorepo options:
  --package <pkg>      Package name (validated against config.yaml packages)

List options:
  --mine, -m           Show only tasks assigned to current developer
  --status, -s <s>     Filter by status (planning, in_progress, review, completed)
  --detail, -d         Show detailed info per task

Status options:
  --mine, -m           Show only tasks assigned to current developer
  --json               Output JSON format (for script consumption)

Examples:
  python3 task.py create "Add login feature" --slug add-login
  python3 task.py create "Add login feature" --slug add-login --package cli
  python3 task.py create "Child task" --slug child --parent .harness-cli/tasks/01-21-parent
  python3 task.py init-context .harness-cli/tasks/01-21-add-login backend
  python3 task.py init-context .harness-cli/tasks/01-21-add-login backend --package cli
  python3 task.py add-context <dir> implement .harness-cli/spec/cli/backend/auth.md "Auth guidelines"
  python3 task.py set-branch <dir> task/add-login
  python3 task.py start .harness-cli/tasks/01-21-add-login
  python3 task.py create-pr                          # Uses current task
  python3 task.py create-pr <dir> --dry-run          # Preview without changes
  python3 task.py finish
  python3 task.py archive add-login
  python3 task.py add-subtask parent-task child-task  # Link existing tasks
  python3 task.py remove-subtask parent-task child-task
  python3 task.py list                               # List all active tasks
  python3 task.py list --detail                      # List with detailed info
  python3 task.py list --mine                        # List my tasks only
  python3 task.py list --mine --status in_progress   # List my in-progress tasks
  python3 task.py status                             # Status dashboard
  python3 task.py status --mine                      # My tasks dashboard
  python3 task.py status --json                      # JSON output
""")


# =============================================================================
# Main Entry
# =============================================================================

def main() -> int:
    """CLI entry point."""
    parser = argparse.ArgumentParser(
        description="Task Management Script for Multi-Agent Pipeline",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    subparsers = parser.add_subparsers(dest="command", help="Commands")

    # create
    p_create = subparsers.add_parser("create", help="Create new task")
    p_create.add_argument("title", help="Task title")
    p_create.add_argument("--slug", "-s", help="Task slug")
    p_create.add_argument("--assignee", "-a", help="Assignee developer")
    p_create.add_argument("--priority", "-p", default="P2", help="Priority (P0-P3)")
    p_create.add_argument("--description", "-d", help="Task description")
    p_create.add_argument("--parent", help="Parent task directory (establishes subtask link)")
    p_create.add_argument("--package", help="Package name for monorepo projects")

    # init-context
    p_init = subparsers.add_parser("init-context", help="Initialize context files")
    p_init.add_argument("dir", help="Task directory")
    p_init.add_argument("type", help="Dev type: backend|frontend|fullstack|test|docs")
    p_init.add_argument("--package", help="Package name for monorepo projects")

    # add-context
    p_add = subparsers.add_parser("add-context", help="Add context entry")
    p_add.add_argument("dir", help="Task directory")
    p_add.add_argument("file", help="JSONL file (implement|check|debug)")
    p_add.add_argument("path", help="File path to add")
    p_add.add_argument("reason", nargs="?", help="Reason for adding")

    # validate
    p_validate = subparsers.add_parser("validate", help="Validate context files")
    p_validate.add_argument("dir", help="Task directory")

    # list-context
    p_listctx = subparsers.add_parser("list-context", help="List context entries")
    p_listctx.add_argument("dir", help="Task directory")

    # start
    p_start = subparsers.add_parser("start", help="Set current task")
    p_start.add_argument("dir", help="Task directory")

    # finish
    subparsers.add_parser("finish", help="Clear current task")

    # set-branch
    p_branch = subparsers.add_parser("set-branch", help="Set git branch")
    p_branch.add_argument("dir", help="Task directory")
    p_branch.add_argument("branch", help="Branch name")

    # set-base-branch
    p_base = subparsers.add_parser("set-base-branch", help="Set PR target branch")
    p_base.add_argument("dir", help="Task directory")
    p_base.add_argument("base_branch", help="Base branch name (PR target)")

    # set-scope
    p_scope = subparsers.add_parser("set-scope", help="Set scope")
    p_scope.add_argument("dir", help="Task directory")
    p_scope.add_argument("scope", help="Scope name")

    # mark-kb
    p_markkb = subparsers.add_parser(
        "mark-kb",
        help="Set kb_status on a task (needed | updated | not-required)",
    )
    p_markkb.add_argument(
        "status",
        help="New kb_status value: needed | updated | not-required",
    )
    p_markkb.add_argument(
        "task",
        nargs="?",
        help="Task name or dir (defaults to current task)",
    )

    # create-pr
    p_pr = subparsers.add_parser("create-pr", help="Create PR")
    p_pr.add_argument("dir", nargs="?", help="Task directory")
    p_pr.add_argument("--dry-run", action="store_true", help="Dry run mode")

    # archive
    p_archive = subparsers.add_parser("archive", help="Archive task")
    p_archive.add_argument("name", help="Task name")
    p_archive.add_argument("--no-commit", action="store_true", help="Skip auto git commit after archive")

    # list
    p_list = subparsers.add_parser("list", help="List tasks")
    p_list.add_argument("--mine", "-m", action="store_true", help="My tasks only")
    p_list.add_argument("--status", "-s", help="Filter by status")
    p_list.add_argument("--detail", "-d", action="store_true", help="Show detailed info per task")

    # status
    p_status = subparsers.add_parser("status", help="Task status dashboard")
    p_status.add_argument("--mine", "-m", action="store_true", help="My tasks only")
    p_status.add_argument("--json", action="store_true", help="Output JSON format")

    # add-subtask
    p_addsub = subparsers.add_parser("add-subtask", help="Link child task to parent")
    p_addsub.add_argument("parent_dir", help="Parent task directory")
    p_addsub.add_argument("child_dir", help="Child task directory")

    # remove-subtask
    p_rmsub = subparsers.add_parser("remove-subtask", help="Unlink child task from parent")
    p_rmsub.add_argument("parent_dir", help="Parent task directory")
    p_rmsub.add_argument("child_dir", help="Child task directory")

    # list-archive
    p_listarch = subparsers.add_parser("list-archive", help="List archived tasks")
    p_listarch.add_argument("month", nargs="?", help="Month (YYYY-MM)")

    # weekly-report
    p_weekly = subparsers.add_parser(
        "weekly-report",
        help="Generate personal weekly report for the current ISO week",
    )
    p_weekly.add_argument("--week", help="ISO week, e.g. 2026-W15 (default: current week)")
    p_weekly.add_argument("--dev", help="Developer name (default: current developer)")

    args = parser.parse_args()

    if not args.command:
        show_usage()
        return 1

    commands = {
        "create": cmd_create,
        "init-context": cmd_init_context,
        "add-context": cmd_add_context,
        "validate": cmd_validate,
        "list-context": cmd_list_context,
        "start": cmd_start,
        "finish": cmd_finish,
        "set-branch": cmd_set_branch,
        "set-base-branch": cmd_set_base_branch,
        "set-scope": cmd_set_scope,
        "mark-kb": cmd_mark_kb,
        "create-pr": cmd_create_pr,
        "archive": cmd_archive,
        "add-subtask": cmd_add_subtask,
        "remove-subtask": cmd_remove_subtask,
        "list": cmd_list,
        "status": cmd_status,
        "list-archive": cmd_list_archive,
        "weekly-report": cmd_weekly_report,
    }

    if args.command in commands:
        return commands[args.command](args)
    else:
        show_usage()
        return 1


if __name__ == "__main__":
    sys.exit(main())
