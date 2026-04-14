# Workspace Index

> AI development session records across all developers.

---

## Active Developers

<!-- @@@auto:developers -->
| Developer | Current Tasks | Status | Last Active |
|-----------|--------------|--------|-------------|
| (auto-populated on task finish) | - | - | - |
<!-- @@@/auto:developers -->

---

## Structure

```
workspace/
├── index.md              # This file - developer overview
└── {developer}/
    ├── index.md          # Session history and stats
    └── journal-N.md      # Session records (auto-recorded on task finish)
```

Sessions are recorded automatically when tasks are completed via `task.py finish`.
Journal files rotate at 2000 lines.
