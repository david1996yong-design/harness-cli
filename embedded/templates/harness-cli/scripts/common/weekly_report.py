#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Weekly personal report generator.

Aggregates facts for a given ISO week (Monday–Sunday):
    - Tasks: active (created/touched this week) + archived (completedAt this week)
    - Commits: git log filtered by author and date range
    - Journal: ^## Session headings with matching Date line
    - KB/spec changes: git diff --stat restricted to kb/ and spec/ paths

Writes a Markdown report to:
    .harness-cli/workspace/{dev}/reports/{YYYY}-W{NN}.md

Idempotent: re-running the same week overwrites the same file.
Facts only; AI narrative is appended separately by /hc:weekly-review.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from datetime import date, datetime, timedelta
from pathlib import Path

from .git import run_git
from .io import read_json
from .paths import (
    DIR_ARCHIVE,
    DIR_TASKS,
    DIR_WORKFLOW,
    DIR_WORKSPACE,
    FILE_JOURNAL_PREFIX,
    FILE_TASK_JSON,
    get_developer,
    get_repo_root,
    get_tasks_dir,
)


AI_PLACEHOLDER = "<!-- AI 总结：运行 /hc:weekly-review 生成 -->"
AI_ANCHOR = "## AI 总结"


# =============================================================================
# Developer identity validation
# =============================================================================

def _validate_developer(name: str) -> str:
    """Reject values that would escape workspace/<dev>/ via path traversal.

    Developer names are used as path components; if a malicious/careless
    --dev value contains separators or .., subsequent mkdir/write_text
    operations would follow them outside the intended workspace.
    """
    if not name or name in (".", ".."):
        raise ValueError(f"Invalid developer name: {name!r}")
    if name.startswith("."):
        raise ValueError(f"Developer name must not start with a dot: {name!r}")
    if any(ch in name for ch in ("/", "\\")) or ".." in name:
        raise ValueError(f"Developer name contains path separators: {name!r}")
    return name


# =============================================================================
# Week math
# =============================================================================

def resolve_week(week_arg: str | None) -> tuple[int, int, date, date]:
    """Resolve an ISO week into (year, week, monday, sunday).

    Accepts:
        None        — current ISO week
        YYYY-Www    — explicit ISO week (e.g. "2026-W15")
    """
    if week_arg:
        m = re.fullmatch(r"(\d{4})-W(\d{1,2})", week_arg.strip())
        if not m:
            raise ValueError(f"Invalid --week format: {week_arg!r}, expected YYYY-Www")
        year = int(m.group(1))
        week = int(m.group(2))
    else:
        today = date.today()
        iso = today.isocalendar()
        year = iso.year
        week = iso.week

    # ISO: Monday is day 1, Sunday is day 7
    monday = date.fromisocalendar(year, week, 1)
    sunday = date.fromisocalendar(year, week, 7)
    return year, week, monday, sunday


# =============================================================================
# Task aggregation
# =============================================================================

@dataclass
class TaskRow:
    slug: str
    title: str
    priority: str
    kb_status: str
    branch: str
    group: str  # "archived" | "in_progress" | "new"


def _parse_date(value: str | None) -> date | None:
    if not value:
        return None
    s = str(value).strip()
    # Accept either YYYY-MM-DD or full ISO with time
    try:
        return datetime.fromisoformat(s.replace("Z", "+00:00")).date()
    except ValueError:
        try:
            return datetime.strptime(s[:10], "%Y-%m-%d").date()
        except ValueError:
            return None


def _task_row(data: dict, group: str) -> TaskRow:
    return TaskRow(
        slug=data.get("name") or data.get("id") or "unknown",
        title=data.get("title") or "(no title)",
        priority=data.get("priority") or "P2",
        kb_status=data.get("kb_status") or "-",
        branch=data.get("branch") or "-",
        group=group,
    )


def _iter_archive_months(repo_root: Path, monday: date, sunday: date) -> list[Path]:
    """Return archive month directories that could contain tasks for this week."""
    archive_root = repo_root / DIR_WORKFLOW / DIR_TASKS / DIR_ARCHIVE
    if not archive_root.is_dir():
        return []

    months_wanted = set()
    cur = date(monday.year, monday.month, 1)
    end = date(sunday.year, sunday.month, 1)
    while cur <= end:
        months_wanted.add(f"{cur.year:04d}-{cur.month:02d}")
        # advance by a month
        if cur.month == 12:
            cur = date(cur.year + 1, 1, 1)
        else:
            cur = date(cur.year, cur.month + 1, 1)

    return [archive_root / m for m in sorted(months_wanted) if (archive_root / m).is_dir()]


def collect_tasks(
    repo_root: Path,
    developer: str,
    monday: date,
    sunday: date,
    is_current_week: bool,
) -> list[TaskRow]:
    """Return tasks relevant to this week, grouped archived / in_progress / new.

    For historical weeks, only tasks that actually started/ended in the window
    are shown. "Right-now in_progress" status is only meaningful for the
    current week.
    """
    rows: list[TaskRow] = []

    # Archived: completedAt within range
    for month_dir in _iter_archive_months(repo_root, monday, sunday):
        for task_dir in sorted(month_dir.iterdir()):
            if not task_dir.is_dir():
                continue
            data = read_json(task_dir / FILE_TASK_JSON) or {}
            if data.get("assignee") and data.get("assignee") != developer:
                continue
            completed_at = _parse_date(data.get("completedAt"))
            if completed_at and monday <= completed_at <= sunday:
                rows.append(_task_row(data, "archived"))

    # Active tasks (non-archive)
    tasks_dir = get_tasks_dir(repo_root)
    if tasks_dir.is_dir():
        for task_dir in sorted(tasks_dir.iterdir()):
            if not task_dir.is_dir() or task_dir.name == DIR_ARCHIVE:
                continue
            data = read_json(task_dir / FILE_TASK_JSON) or {}
            if data.get("assignee") and data.get("assignee") != developer:
                continue
            status = data.get("status") or ""
            created_at = _parse_date(data.get("createdAt"))
            created_in_week = bool(created_at and monday <= created_at <= sunday)

            if status in ("in_progress", "review"):
                # "Currently ongoing" only makes sense for the current week's report.
                # For historical weeks, only include if the task was actually
                # created in that week.
                if is_current_week or created_in_week:
                    rows.append(_task_row(data, "in_progress"))
            elif created_in_week:
                # New this week but not started yet
                rows.append(_task_row(data, "new"))

    return rows


# =============================================================================
# Commits
# =============================================================================

def _git_author_pattern(repo_root: Path, developer: str) -> str:
    """Build a regex matching this developer's git identity.

    The `.harness-cli/.developer` name is a workspace identifier; commits
    are attributed by `user.name`/`user.email` which frequently differ
    from it. Prefer the actual git identity and fall back to the workspace
    name only if neither is configured.
    """
    patterns: list[str] = []
    for key in ("user.name", "user.email"):
        code, out, _ = run_git(["config", "--get", key], cwd=repo_root)
        val = out.strip()
        if code == 0 and val:
            patterns.append(re.escape(val))
    if not patterns:
        patterns.append(re.escape(developer))
    return "|".join(patterns)


def collect_commits(
    repo_root: Path,
    developer: str,
    monday: date,
    sunday: date,
) -> list[tuple[date, str, str]]:
    """Return [(date, hash, subject), ...] for commits by developer this week."""
    until = sunday + timedelta(days=1)  # git --until is exclusive at midnight boundary
    author_pattern = _git_author_pattern(repo_root, developer)
    args = [
        "log",
        "-E",  # extended regex so `|` alternation works in --author
        f"--since={monday.isoformat()}",
        f"--until={until.isoformat()}",
        f"--author={author_pattern}",
        "--pretty=format:%h%x09%cs%x09%s",
    ]
    code, out, _ = run_git(args, cwd=repo_root)
    if code != 0 or not out.strip():
        return []

    result: list[tuple[date, str, str]] = []
    for line in out.splitlines():
        parts = line.split("\t", 2)
        if len(parts) != 3:
            continue
        h, cs, subj = parts
        d = _parse_date(cs)
        if d:
            result.append((d, h.strip(), subj.strip()))
    result.sort(key=lambda r: (r[0], r[1]))
    return result


# =============================================================================
# Journal
# =============================================================================

def collect_journal_titles(
    repo_root: Path,
    developer: str,
    monday: date,
    sunday: date,
) -> list[tuple[date, str]]:
    """Return [(date, title), ...] from journal ## Session headings in range."""
    workspace = repo_root / DIR_WORKFLOW / DIR_WORKSPACE / developer
    if not workspace.is_dir():
        return []

    heading_re = re.compile(r"^##\s+(.*\S)\s*$")
    date_re = re.compile(r"^\*\*Date\*\*:\s*(\d{4}-\d{2}-\d{2})")

    results: list[tuple[date, str]] = []
    for jfile in sorted(workspace.glob(f"{FILE_JOURNAL_PREFIX}*.md")):
        try:
            text = jfile.read_text(encoding="utf-8")
        except OSError:
            continue

        current_title: str | None = None
        for raw in text.splitlines():
            mh = heading_re.match(raw)
            if mh:
                current_title = mh.group(1).strip()
                continue
            if current_title:
                md = date_re.match(raw.strip())
                if md:
                    d = _parse_date(md.group(1))
                    if d and monday <= d <= sunday:
                        results.append((d, current_title))
                    current_title = None  # only pair with first Date line

    results.sort(key=lambda r: r[0])
    return results


# =============================================================================
# KB / spec diff
# =============================================================================

def collect_kb_changes(
    repo_root: Path,
    monday: date,
    sunday: date,
) -> dict[str, int]:
    """Return {prefix: file_count_changed} for kb/prd, kb/tech, spec/."""
    until = sunday + timedelta(days=1)
    args = [
        "log",
        f"--since={monday.isoformat()}",
        f"--until={until.isoformat()}",
        "--name-only",
        "--pretty=format:",
    ]
    code, out, _ = run_git(args, cwd=repo_root)
    if code != 0:
        return {"kb/prd": 0, "kb/tech": 0, "spec": 0}

    seen: dict[str, set[str]] = {"kb/prd": set(), "kb/tech": set(), "spec": set()}
    for raw in out.splitlines():
        p = raw.strip()
        if not p:
            continue
        if ".harness-cli/kb/prd/" in p:
            seen["kb/prd"].add(p)
        elif ".harness-cli/kb/tech/" in p:
            seen["kb/tech"].add(p)
        elif ".harness-cli/spec/" in p:
            seen["spec"].add(p)

    return {k: len(v) for k, v in seen.items()}


# =============================================================================
# Rendering
# =============================================================================

def _render_task_group(rows: list[TaskRow], group: str, heading: str) -> list[str]:
    subset = [r for r in rows if r.group == group]
    if not subset:
        return []
    lines = [f"### {heading}", ""]
    lines.append("| slug | title | priority | kb_status | branch |")
    lines.append("|------|-------|----------|-----------|--------|")
    for r in subset:
        lines.append(f"| `{r.slug}` | {r.title} | {r.priority} | {r.kb_status} | {r.branch} |")
    lines.append("")
    return lines


def render_report(
    year: int,
    week: int,
    monday: date,
    sunday: date,
    developer: str,
    tasks: list[TaskRow],
    commits: list[tuple[date, str, str]],
    journal: list[tuple[date, str]],
    kb_changes: dict[str, int],
    generated_at: datetime,
) -> str:
    lines: list[str] = []
    lines.append(f"# 周报 {year}-W{week:02d}")
    lines.append("")
    lines.append(f"- **开发者**: {developer}")
    lines.append(f"- **周区间**: {monday.isoformat()} ~ {sunday.isoformat()}")
    lines.append(f"- **生成时间**: {generated_at.strftime('%Y-%m-%d %H:%M:%S')}")
    lines.append("")

    total_tasks = len(tasks)
    total_commits = len(commits)
    if total_tasks == 0 and total_commits == 0 and not journal:
        lines.append("> 本周安静：没有任务活动、没有 commits、也没有 journal 记录。")
        lines.append("")
        lines.append(AI_ANCHOR)
        lines.append("")
        lines.append(AI_PLACEHOLDER)
        lines.append("")
        return "\n".join(lines)

    # Tasks
    lines.append("## 本周任务")
    lines.append("")
    task_blocks: list[str] = []
    task_blocks += _render_task_group(tasks, "archived", "已归档")
    task_blocks += _render_task_group(tasks, "in_progress", "进行中")
    task_blocks += _render_task_group(tasks, "new", "新建未开始")
    if task_blocks:
        lines.extend(task_blocks)
    else:
        lines.append("（无）")
        lines.append("")

    # Commits
    lines.append("## Commits")
    lines.append("")
    if commits:
        last_day: date | None = None
        for d, h, subj in commits:
            if d != last_day:
                lines.append(f"**{d.isoformat()}**")
                last_day = d
            lines.append(f"- `{h}` {subj}")
        lines.append("")
    else:
        lines.append("（无）")
        lines.append("")

    # Journal
    lines.append("## Journal")
    lines.append("")
    if journal:
        for d, title in journal:
            lines.append(f"- {d.isoformat()} — {title}")
        lines.append("")
    else:
        lines.append("（无）")
        lines.append("")

    # KB / spec
    lines.append("## KB / spec 变更")
    lines.append("")
    lines.append(f"- kb/prd: {kb_changes.get('kb/prd', 0)} 个文件")
    lines.append(f"- kb/tech: {kb_changes.get('kb/tech', 0)} 个文件")
    lines.append(f"- spec: {kb_changes.get('spec', 0)} 个文件")
    lines.append("")

    # AI anchor (always appended, never touched by this script)
    lines.append(AI_ANCHOR)
    lines.append("")
    lines.append(AI_PLACEHOLDER)
    lines.append("")

    return "\n".join(lines)


# =============================================================================
# Entry
# =============================================================================

def weekly_report_path(repo_root: Path, developer: str, year: int, week: int) -> Path:
    return (
        repo_root
        / DIR_WORKFLOW
        / DIR_WORKSPACE
        / developer
        / "reports"
        / f"{year:04d}-W{week:02d}.md"
    )


def generate_weekly_report(
    week_arg: str | None = None,
    dev_override: str | None = None,
    repo_root: Path | None = None,
) -> tuple[Path, str]:
    """Generate report and return (path, content). Writes file to disk."""
    if repo_root is None:
        repo_root = get_repo_root()

    developer = dev_override or get_developer(repo_root)
    if not developer:
        raise RuntimeError("Developer is not initialized (.harness-cli/.developer missing)")
    developer = _validate_developer(developer)

    year, week, monday, sunday = resolve_week(week_arg)
    today_iso = date.today().isocalendar()
    is_current_week = (today_iso.year == year and today_iso.week == week)

    tasks = collect_tasks(repo_root, developer, monday, sunday, is_current_week)
    commits = collect_commits(repo_root, developer, monday, sunday)
    journal = collect_journal_titles(repo_root, developer, monday, sunday)
    kb_changes = collect_kb_changes(repo_root, monday, sunday)

    content = render_report(
        year=year,
        week=week,
        monday=monday,
        sunday=sunday,
        developer=developer,
        tasks=tasks,
        commits=commits,
        journal=journal,
        kb_changes=kb_changes,
        generated_at=datetime.now(),
    )

    out_path = weekly_report_path(repo_root, developer, year, week)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(content, encoding="utf-8")
    return out_path, content


# =============================================================================
# Session-start hook helper
# =============================================================================

def should_remind_weekly_report(repo_root: Path | None = None, today: date | None = None) -> str | None:
    """Return a one-line reminder string if Sun/Mon and the target week's report is missing.

    The "target week" semantics:
      - Sunday (weekday=7): current week — you're finishing the week; remind to generate its retro.
      - Monday  (weekday=1): previous week — the week just ended; remind to retro it.

    Returns None on all other days or when the developer is not initialized.
    """
    if repo_root is None:
        repo_root = get_repo_root()
    developer = get_developer(repo_root)
    if not developer:
        return None

    if today is None:
        today = date.today()

    # ISO: Monday=1 .. Sunday=7
    weekday = today.isocalendar().weekday
    if weekday not in (1, 7):
        return None

    # Monday → look back at last week. Sunday → current week.
    probe = today - timedelta(days=1) if weekday == 1 else today
    iso = probe.isocalendar()
    path = weekly_report_path(repo_root, developer, iso.year, iso.week)
    if path.exists():
        return None

    target = f"{iso.year}-W{iso.week:02d}"
    return (
        f"[!] {target} 周报尚未生成，运行 "
        f"`python3 .harness-cli/scripts/task.py weekly-report --week {target}` 生成"
    )
