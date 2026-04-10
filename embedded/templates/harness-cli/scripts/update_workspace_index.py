#!/usr/bin/env python3
"""
更新 workspace/index.md 中的 Active Developers 表。

扫描 workspace/ 下所有开发者子目录，查找各开发者分配的活跃任务，
并用自动生成的表格替换 index.md 中的标记区域。

Usage:
    python3 update_workspace_index.py
"""

from __future__ import annotations

import sys
from collections import Counter
from datetime import datetime
from pathlib import Path

from common.paths import (
    DIR_WORKFLOW,
    DIR_WORKSPACE,
    FILE_JOURNAL_PREFIX,
    get_repo_root,
    get_tasks_dir,
)
from common.tasks import iter_active_tasks


# =============================================================================
# Constants
# =============================================================================

MARKER_START = "<!-- @@@auto:developers -->"
MARKER_END = "<!-- @@@/auto:developers -->"


# =============================================================================
# Helper Functions
# =============================================================================

def _list_developers(workspace_dir: Path) -> list[str]:
    """列出 workspace/ 下所有开发者子目录名称。

    排除 index.md 和非目录文件。

    Args:
        workspace_dir: workspace 目录路径。

    Returns:
        排序后的开发者名称列表。
    """
    if not workspace_dir.is_dir():
        return []

    developers = []
    for entry in sorted(workspace_dir.iterdir()):
        if entry.is_dir():
            developers.append(entry.name)
    return developers


def _get_developer_tasks(developer: str, tasks_dir: Path) -> list[dict]:
    """获取分配给指定开发者的所有活跃任务。

    Args:
        developer: 开发者名称。
        tasks_dir: 任务目录路径。

    Returns:
        任务字典列表，每个包含 title 和 status。
    """
    results = []
    for task in iter_active_tasks(tasks_dir):
        if (task.assignee or "") == developer:
            results.append({
                "title": task.title,
                "status": task.status,
            })
    return results


def _get_last_active(dev_dir: Path) -> str:
    """从 journal 文件的修改时间推断开发者最后活跃时间。

    Args:
        dev_dir: 开发者 workspace 目录路径。

    Returns:
        日期字符串 (YYYY-MM-DD) 或 "-"。
    """
    latest_mtime = 0.0

    for f in dev_dir.glob(f"{FILE_JOURNAL_PREFIX}*.md"):
        if f.is_file():
            mtime = f.stat().st_mtime
            if mtime > latest_mtime:
                latest_mtime = mtime

    if latest_mtime > 0:
        return datetime.fromtimestamp(latest_mtime).strftime("%Y-%m-%d")
    return "-"


def _format_status_summary(tasks: list[dict]) -> str:
    """汇总任务状态。

    Args:
        tasks: 任务列表。

    Returns:
        状态汇总字符串，如 "2 in_progress, 1 planning"，无任务时返回 "-"。
    """
    if not tasks:
        return "-"

    counter: Counter[str] = Counter()
    for t in tasks:
        counter[t["status"]] += 1

    parts = [f"{count} {status}" for status, count in sorted(counter.items())]
    return ", ".join(parts)


def _build_developers_table(workspace_dir: Path, tasks_dir: Path) -> str:
    """构建 Active Developers 表格内容。

    Args:
        workspace_dir: workspace 目录路径。
        tasks_dir: 任务目录路径。

    Returns:
        Markdown 表格字符串。
    """
    developers = _list_developers(workspace_dir)

    lines = [
        "| Developer | Current Tasks | Status | Last Active |",
        "|-----------|--------------|--------|-------------|",
    ]

    if not developers:
        lines.append("| (none yet) | - | - | - |")
        return "\n".join(lines)

    for dev in developers:
        dev_dir = workspace_dir / dev
        tasks = _get_developer_tasks(dev, tasks_dir)

        # 任务标题
        if tasks:
            task_titles = ", ".join(t["title"] for t in tasks)
        else:
            task_titles = "-"

        # 状态汇总
        status_summary = _format_status_summary(tasks)

        # 最后活跃时间
        last_active = _get_last_active(dev_dir)

        lines.append(f"| {dev} | {task_titles} | {status_summary} | {last_active} |")

    return "\n".join(lines)


# =============================================================================
# Core Function
# =============================================================================

def update_workspace_index(repo_root: Path | None = None) -> bool:
    """更新 workspace/index.md 中的 Active Developers 表。

    使用标记注释定位替换区域，生成的表格包含开发者、任务、状态等信息。
    脚本幂等：多次运行结果一致。

    Args:
        repo_root: 仓库根目录路径，默认自动检测。

    Returns:
        True 成功，False 失败。
    """
    if repo_root is None:
        repo_root = get_repo_root()

    workspace_dir = repo_root / DIR_WORKFLOW / DIR_WORKSPACE
    tasks_dir = get_tasks_dir(repo_root)
    index_file = workspace_dir / "index.md"

    # 检查 index.md 是否存在
    if not index_file.is_file():
        print(f"Warning: {index_file} not found, skipping update.", file=sys.stderr)
        return False

    content = index_file.read_text(encoding="utf-8")

    # 检查标记是否存在
    if MARKER_START not in content or MARKER_END not in content:
        print(
            f"Warning: auto markers not found in {index_file}, skipping update.",
            file=sys.stderr,
        )
        return False

    # 构建新的表格
    table = _build_developers_table(workspace_dir, tasks_dir)

    # 替换标记区域之间的内容
    lines = content.splitlines()
    new_lines: list[str] = []
    in_marker = False

    for line in lines:
        if MARKER_START in line:
            new_lines.append(line)
            new_lines.append(table)
            in_marker = True
            continue

        if MARKER_END in line:
            in_marker = False
            new_lines.append(line)
            continue

        if in_marker:
            # 跳过旧内容
            continue

        new_lines.append(line)

    new_content = "\n".join(new_lines)
    # 保持文件末尾换行符
    if content.endswith("\n") and not new_content.endswith("\n"):
        new_content += "\n"

    try:
        index_file.write_text(new_content, encoding="utf-8")
    except (OSError, IOError) as e:
        print(f"Error: Failed to write {index_file}: {e}", file=sys.stderr)
        return False

    print(f"[OK] Updated {index_file}")
    return True


# =============================================================================
# Main Entry
# =============================================================================

def main() -> None:
    """CLI entry point."""
    success = update_workspace_index()
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
