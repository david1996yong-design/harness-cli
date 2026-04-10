#!/usr/bin/env python3
"""
Initialize developer for workflow.

Usage:
    python3 init_developer.py <developer-name>

This creates:
    - .harness-cli/.developer file with developer info
    - .harness-cli/workspace/<name>/ directory structure
"""

from __future__ import annotations

import sys

from common.paths import (
    DIR_WORKFLOW,
    FILE_DEVELOPER,
    get_developer,
)
from common.developer import init_developer
from update_workspace_index import update_workspace_index


def main() -> None:
    """CLI entry point."""
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <developer-name>")
        print()
        print("Example:")
        print(f"  {sys.argv[0]} john")
        sys.exit(1)

    name = sys.argv[1]

    # Check if already initialized
    existing = get_developer()
    if existing:
        print(f"Developer already initialized: {existing}")
        print()
        print(f"To reinitialize, remove {DIR_WORKFLOW}/{FILE_DEVELOPER} first")
        sys.exit(0)

    if init_developer(name):
        # 刷新全局 workspace/index.md
        update_workspace_index()
        sys.exit(0)
    else:
        sys.exit(1)


if __name__ == "__main__":
    main()
