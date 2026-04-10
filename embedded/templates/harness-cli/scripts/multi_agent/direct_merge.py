#!/usr/bin/env python3
"""
Multi-Agent Pipeline: Direct Merge.

Usage:
    python3 direct_merge.py [task-dir] [--dry-run]

This script:
1. Stages and commits all changes (excluding workspace, .agent-log, .session-id)
2. Pushes feature branch to remote
3. Merges feature branch into target branch (--no-ff) via the main repository
4. Pushes target branch
5. Deletes remote feature branch
6. Updates task.json with status="completed" and current_phase

Note: This is an alternative to create_pr.py for small features that don't
need a PR review cycle. The merge is performed in the main repository since
worktrees cannot checkout other branches.

Conflict handling: On merge conflict, the script exits with an error, prints
clear instructions, and leaves the worktree intact for manual resolution.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

import _bootstrap  # noqa: F401 — adds parent scripts/ dir to sys.path

from common.git import run_git
from common.io import read_json, write_json
from common.log import Colors, log_info, log_success, log_warn, log_error
from common.paths import (
    DIR_WORKFLOW,
    FILE_TASK_JSON,
    get_current_task,
    get_repo_root,
)
from common.phase import get_phase_for_action


# =============================================================================
# Main
# =============================================================================


def main() -> int:
    """Main entry point."""
    parser = argparse.ArgumentParser(description="Multi-Agent Pipeline: Direct Merge")
    parser.add_argument("dir", nargs="?", help="Task directory")
    parser.add_argument(
        "--dry-run", action="store_true", help="Show what would be done"
    )

    args = parser.parse_args()
    repo_root = get_repo_root()

    # =============================================================================
    # Get Task Directory
    # =============================================================================
    target_dir = args.dir
    if not target_dir:
        # Try to get from .current-task
        current_task = get_current_task(repo_root)
        if current_task:
            target_dir = current_task

    if not target_dir:
        log_error("No task directory specified and no current task set")
        print("Usage: python3 direct_merge.py [task-dir] [--dry-run]")
        return 1

    # Support relative paths
    if not target_dir.startswith("/"):
        target_dir_path = repo_root / target_dir
    else:
        target_dir_path = Path(target_dir)

    task_json = target_dir_path / FILE_TASK_JSON
    if not task_json.is_file():
        log_error(f"task.json not found at {target_dir_path}")
        return 1

    # =============================================================================
    # Read Task Config
    # =============================================================================
    print(f"{Colors.BLUE}=== Direct Merge ==={Colors.NC}")
    if args.dry_run:
        print(
            f"{Colors.YELLOW}[DRY-RUN MODE] No actual changes will be made{Colors.NC}"
        )
    print()

    task_data = read_json(task_json)
    if not task_data:
        log_error("Failed to read task.json")
        return 1

    task_name = task_data.get("name", "")
    merge_target = task_data.get("merge_target")
    base_branch = task_data.get("base_branch", "main")
    scope = task_data.get("scope", "core")
    dev_type = task_data.get("dev_type", "feature")

    # Resolve merge target (fallback to base_branch)
    if not merge_target:
        merge_target = base_branch
        log_warn(f"merge_target not set in task.json, using base_branch: {merge_target}")

    # Map dev_type to commit prefix
    prefix_map = {
        "feature": "feat",
        "frontend": "feat",
        "backend": "feat",
        "fullstack": "feat",
        "bugfix": "fix",
        "fix": "fix",
        "refactor": "refactor",
        "docs": "docs",
        "test": "test",
    }
    commit_prefix = prefix_map.get(dev_type, "feat")

    print(f"Task: {task_name}")
    print(f"Merge target: {merge_target}")
    print(f"Scope: {scope}")
    print(f"Commit prefix: {commit_prefix}")
    print()

    # Get current branch (feature branch)
    _, branch_out, _ = run_git(["branch", "--show-current"])
    current_branch = branch_out.strip()
    print(f"Current branch: {current_branch}")

    if current_branch == merge_target:
        log_error(
            f"Current branch '{current_branch}' is the same as merge target '{merge_target}'. "
            "Cannot merge a branch into itself."
        )
        return 1

    # =============================================================================
    # Step 1: Stage and Commit
    # =============================================================================
    print(f"\n{Colors.YELLOW}Step 1: Staging and committing changes...{Colors.NC}")

    # Stage changes
    run_git(["add", "-A"])

    # Exclude workspace and temp files (same as create_pr.py)
    run_git(["reset", f"{DIR_WORKFLOW}/workspace/"])
    run_git(["reset", ".agent-log", ".session-id"])

    # Check if there are staged changes
    ret, _, _ = run_git(["diff", "--cached", "--quiet"])
    has_staged_changes = ret != 0

    if not has_staged_changes:
        print("No staged changes to commit")

        # Check for unpushed commits
        ret, log_out, _ = run_git(
            ["log", f"origin/{current_branch}..HEAD", "--oneline"]
        )
        unpushed = len([line for line in log_out.splitlines() if line.strip()])

        if unpushed == 0:
            if args.dry_run:
                run_git(["reset", "HEAD"])
            log_warn("No changes and no unpushed commits")
            # Still proceed - there might be already-pushed commits to merge
            ret2, log_out2, _ = run_git(
                ["log", f"{merge_target}..HEAD", "--oneline"]
            )
            ahead = len([line for line in log_out2.splitlines() if line.strip()])
            if ahead == 0:
                log_error("No changes to merge")
                return 1
            print(f"Found {ahead} commit(s) ahead of {merge_target}")
        else:
            print(f"Found {unpushed} unpushed commit(s)")
    else:
        # Commit changes
        commit_msg = f"{commit_prefix}({scope}): {task_name}"

        if args.dry_run:
            print(f"[DRY-RUN] Would commit with message: {commit_msg}")
            _, staged_out, _ = run_git(["diff", "--cached", "--name-only"])
            for line in staged_out.splitlines():
                print(f"  - {line}")
        else:
            run_git(["commit", "-m", commit_msg])
            log_success(f"Committed: {commit_msg}")

    # =============================================================================
    # Step 2: Push feature branch to remote
    # =============================================================================
    print(f"\n{Colors.YELLOW}Step 2: Pushing feature branch to remote...{Colors.NC}")

    if args.dry_run:
        print(f"[DRY-RUN] Would push to: origin/{current_branch}")
    else:
        ret, _, err = run_git(["push", "-u", "origin", current_branch])
        if ret != 0:
            log_error(f"Failed to push feature branch: {err}")
            return 1
        log_success(f"Pushed to origin/{current_branch}")

    # =============================================================================
    # Step 3: Merge into target branch (in main repo via fetch + merge)
    # =============================================================================
    print(f"\n{Colors.YELLOW}Step 3: Merging into {merge_target}...{Colors.NC}")

    # Determine the main repository path.
    # In a worktree, .git is a file pointing to the main repo's .git/worktrees/<name>.
    # We need the main repo root to perform the merge (worktrees can't checkout other branches).
    git_path = repo_root / ".git"
    if git_path.is_file():
        # This is a worktree - read the gitdir reference to find main repo
        gitdir_content = git_path.read_text(encoding="utf-8").strip()
        # Format: "gitdir: /path/to/main/.git/worktrees/<name>"
        if gitdir_content.startswith("gitdir: "):
            gitdir_path = Path(gitdir_content[8:])
            # Navigate up from .git/worktrees/<name> to main repo root
            # .git/worktrees/<name> -> .git -> repo root
            main_repo_git = gitdir_path.parent.parent  # .git dir
            main_repo_root = main_repo_git.parent
        else:
            log_error("Cannot determine main repository path from worktree .git file")
            return 1
    else:
        # Already in main repo
        main_repo_root = repo_root

    log_info(f"Main repository: {main_repo_root}")

    # Verify main repo is valid
    if not (main_repo_root / ".git").is_dir():
        log_error(f"Main repo .git directory not found at {main_repo_root}")
        return 1

    if args.dry_run:
        print(f"[DRY-RUN] Would fetch origin/{current_branch}")
        print(f"[DRY-RUN] Would checkout {merge_target} in main repo")
        print(f"[DRY-RUN] Would merge --no-ff origin/{current_branch}")
    else:
        # Fetch the feature branch in main repo
        ret, _, err = run_git(
            ["fetch", "origin", current_branch], cwd=main_repo_root
        )
        if ret != 0:
            log_error(f"Failed to fetch origin/{current_branch}: {err}")
            return 1

        # Checkout the target branch in main repo
        ret, _, err = run_git(
            ["checkout", merge_target], cwd=main_repo_root
        )
        if ret != 0:
            log_error(
                f"Failed to checkout {merge_target} in main repo: {err}\n"
                f"The target branch may be checked out in another worktree."
            )
            return 1

        # Pull latest changes on target branch
        ret, _, err = run_git(
            ["pull", "--ff-only", "origin", merge_target], cwd=main_repo_root
        )
        if ret != 0:
            log_warn(f"Could not pull latest {merge_target}: {err}")
            # Non-fatal: continue with local state

        # Merge with --no-ff
        commit_prefix_full = f"{commit_prefix}({scope}): {task_name}"
        merge_msg = f"Merge branch '{current_branch}' into {merge_target}\n\n{commit_prefix_full}"

        ret, _, err = run_git(
            ["merge", "--no-ff", f"origin/{current_branch}", "-m", merge_msg],
            cwd=main_repo_root,
        )
        if ret != 0:
            log_error(
                f"Merge conflict! Failed to merge {current_branch} into {merge_target}.\n"
                f"Error: {err}\n"
                f"\n"
                f"To resolve manually:\n"
                f"  cd {main_repo_root}\n"
                f"  git status  # see conflicting files\n"
                f"  # resolve conflicts...\n"
                f"  git add .\n"
                f"  git commit\n"
                f"  git push origin {merge_target}\n"
                f"\n"
                f"Or to abort the merge:\n"
                f"  cd {main_repo_root}\n"
                f"  git merge --abort"
            )
            return 1

        log_success(f"Merged {current_branch} into {merge_target}")

        # Get merge commit hash
        _, merge_hash_out, _ = run_git(
            ["rev-parse", "HEAD"], cwd=main_repo_root
        )
        merge_commit = merge_hash_out.strip()[:12]

    # =============================================================================
    # Step 4: Push target branch
    # =============================================================================
    print(f"\n{Colors.YELLOW}Step 4: Pushing {merge_target}...{Colors.NC}")

    if args.dry_run:
        print(f"[DRY-RUN] Would push {merge_target} to origin")
    else:
        ret, _, err = run_git(
            ["push", "origin", merge_target], cwd=main_repo_root
        )
        if ret != 0:
            log_error(f"Failed to push {merge_target}: {err}")
            return 1
        log_success(f"Pushed {merge_target} to origin")

    # =============================================================================
    # Step 5: Delete remote feature branch
    # =============================================================================
    print(f"\n{Colors.YELLOW}Step 5: Cleaning up remote feature branch...{Colors.NC}")

    if args.dry_run:
        print(f"[DRY-RUN] Would delete remote branch: origin/{current_branch}")
    else:
        ret, _, err = run_git(
            ["push", "origin", "--delete", current_branch], cwd=main_repo_root
        )
        if ret != 0:
            log_warn(f"Could not delete remote branch origin/{current_branch}: {err}")
        else:
            log_success(f"Deleted remote branch: origin/{current_branch}")

    # =============================================================================
    # Step 6: Update task.json
    # =============================================================================
    print(f"\n{Colors.YELLOW}Step 6: Updating task status...{Colors.NC}")

    if args.dry_run:
        print("[DRY-RUN] Would update task.json:")
        print("  status: completed")
        print("  current_phase: (set to direct-merge phase)")
        # Reset staging in dry-run
        run_git(["reset", "HEAD"])
    else:
        # Get the phase number for direct-merge action
        direct_merge_phase = get_phase_for_action(task_json, "direct-merge")
        if not direct_merge_phase:
            direct_merge_phase = 4  # Default fallback

        task_data["status"] = "completed"
        task_data["current_phase"] = direct_merge_phase
        task_data["merge_commit"] = merge_commit

        write_json(task_json, task_data)
        log_success(
            f"Task status updated to 'completed', phase {direct_merge_phase}"
        )

    # =============================================================================
    # Summary
    # =============================================================================
    print()
    print(f"{Colors.GREEN}=== Direct Merge Completed ==={Colors.NC}")
    if not args.dry_run:
        print(f"Merge commit: {merge_commit}")
    print(f"Target branch: {merge_target}")
    print(f"Feature branch: {current_branch}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
