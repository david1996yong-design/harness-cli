Show a comprehensive task status dashboard with intelligent suggestions.

Execute these steps:

1. **Get task status data**:
   ```bash
   python3 ./.harness-cli/scripts/task.py status --json
   ```

2. **Get agent running status** (use registry module to find correct path):
   ```bash
   python3 -c "from harness_cli.scripts.common.registry import registry_list_agents; import json; print(json.dumps({'agents': registry_list_agents()}, indent=2))" 2>/dev/null || python3 -c "
import json
from pathlib import Path
repo = Path('.harness-cli')
# Find registry.json under workspace/<developer>/.agents/
registries = list(repo.glob('workspace/*/.agents/registry.json'))
if registries:
    print(registries[0].read_text())
else:
    print(json.dumps({'agents': []}))
"
   ```

3. **Format as dashboard** using the JSON data:

   Present the information as:

   ### Task Overview

   | Priority | Title | Status | Assignee | Branch |
   |----------|-------|--------|----------|--------|
   | ... | ... | ... | ... | ... |

   ### Summary

   - Total: N tasks
   - By status: planning: X, in_progress: Y, review: Z, completed: W
   - By priority: P0: A, P1: B, P2: C, P3: D

   ### Agent Status

   Show running agents from registry.json (if any).

4. **Provide intelligent suggestions**:

   - If there are P0 tasks not in `in_progress` or `completed` status, warn: "P0 task(s) need attention"
   - If there are tasks in `in_progress` status with a `createdAt` older than 7 days, warn: "Long-running in-progress task(s) detected"
   - If there are no tasks at all, suggest creating one
   - If all tasks are completed, congratulate and suggest archiving

5. **Report the dashboard** to the user in a clear, readable format.
