Archive all completed tasks in one shot.

Execute these steps:

1. **Get all completed tasks** as JSON:
   ```bash
   python3 ./.harness-cli/scripts/task.py status --json
   ```

   Parse the output and extract `by_status.completed[]`. Each entry has a `dir_name` field — that's the argument you pass to `task.py archive`.

2. **If there are no completed tasks**, tell the user "当前没有已完成的任务可归档" and stop.

3. **Preview the archive plan** before acting. Print a short list like:
   ```
   即将归档以下 N 个任务：
     - <dir_name>  <title>
     - ...
   ```
   Do NOT prompt for confirmation — the user invoked this command, so they already opted in.

4. **Archive each task sequentially**:
   ```bash
   python3 ./.harness-cli/scripts/task.py archive <dir_name>
   ```

   Run one archive per task. The default behavior auto-commits each archive (one commit per task), which keeps git history clean.

   If a single archive fails, report the error, skip that task, and continue with the rest. Do NOT abort the whole batch on one failure.

5. **Report summary**:
   ```
   ✅ 已归档 N/M 个任务
   - 成功: <list>
   - 失败: <list with reason> (if any)
   ```

6. **Suggest next step** (only if all archived successfully):
   Tell the user to run `/hc:task-dashboard` to verify the cleaned-up list.

Notes:
- Task name for archive command is the **dir_name** (e.g. `04-10-task-set-priority`), not the `name` or `title` field.
- Each archive runs `git commit` by default — this is intentional, the user wants clean git history after batch archive.
- The `completed` task filter uses the `status` field in task.json, which may not reflect real git state. Trust the status — do NOT re-verify against git.
