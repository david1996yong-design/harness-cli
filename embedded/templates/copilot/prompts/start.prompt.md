---
description: "Harness CLI Copilot prompt: Start Session"
---

# Start Session

Initialize your AI development session and begin working on tasks.

## Workflow Modes

This command supports three workflow modes:

| Mode | Trigger | Purpose |
|------|---------|---------|
| **Dev** (default) | `start` or `start dev` | Build features, handle development tasks |
| **Debug** | `start debug [description]` | Structured bug diagnosis and fix |
| **Arch** | `start arch [module]` | Architecture review and optimization |

---

## Operation Types

| Marker | Meaning | Executor |
|--------|---------|----------|
| `[AI]` | Bash scripts or Task calls executed by AI | You (AI) |
| `[USER]` | Slash commands executed by user | User |

---

## Initialization `[AI]`

### Step 1: Understand Development Workflow

First, read the workflow guide to understand the development process:

```bash
cat .harness-cli/workflow.md
```

**Follow the instructions in workflow.md** - it contains:
- Core principles (Read Before Write, Follow Standards, etc.)
- File system structure
- Development process
- Best practices

### Step 2: Get Current Context

```bash
python3 ./.harness-cli/scripts/get_context.py
```

This shows: developer identity, git status, current task (if any), active tasks.

### Step 3: Read Guidelines Index

```bash
python3 ./.harness-cli/scripts/get_context.py --mode packages
```

This shows available packages and their spec layers. Read the relevant spec indexes:

```bash
cat .harness-cli/spec/<package>/<layer>/index.md   # Package-specific guidelines
cat .harness-cli/spec/guides/index.md              # Thinking guides (always read)
```

> **Important**: The index files are navigation �?they list the actual guideline files (e.g., `error-handling.md`, `conventions.md`, `mock-strategies.md`).
> At this step, just read the indexes to understand what's available.
> When you start actual development, you MUST go back and read the specific guideline files relevant to your task, as listed in the index's Pre-Development Checklist.

### Step 4: Mode Routing

Parse the user's command arguments to determine the workflow mode:

| User Input | Mode | Action |
|------------|------|--------|
| `start` | Dev | Proceed to **Step 5: Report and Ask** |
| `start dev` | Dev | Proceed to **Step 5: Report and Ask** |
| `start debug` | Debug | Jump to **Debug Mode** section |
| `start debug "error message"` | Debug | Jump to **Debug Mode** with initial description |
| `start arch` | Arch | Jump to **Arch Mode** (full project scan) |
| `start arch <module>` | Arch | Jump to **Arch Mode** (scoped to module) |

If the user provided arguments after the command name, extract them:
- **First word** after `start` → mode selector (`debug`, `arch`, or anything else = dev)
- **Remaining text** → mode-specific argument (problem description for debug, module name for arch)

### Step 5: Report and Ask (Dev Mode)

Report what you learned and ask: "What would you like to work on?"

---

## Task Classification (Dev Mode)

When user describes a task, classify it:

| Type | Criteria | Workflow |
|------|----------|----------|
| **Question** | User asks about code, architecture, or how something works | Answer directly |
| **Trivial Fix** | Typo fix, comment update, single-line change | Direct Edit |
| **Simple Task** | Clear goal, 1-2 files, well-defined scope | Quick confirm �?Implement |
| **Complex Task** | Vague goal, multiple files, architectural decisions | **Brainstorm �?Task Workflow** |

### Classification Signals

**Trivial/Simple indicators:**
- User specifies exact file and change
- "Fix the typo in X"
- "Add field Y to component Z"
- Clear acceptance criteria already stated

**Complex indicators:**
- "I want to add a feature for..."
- "Can you help me improve..."
- Mentions multiple areas or systems
- No clear implementation path
- User seems unsure about approach

### Decision Rule

> **If in doubt, use Brainstorm + Task Workflow.**
>
> Task Workflow ensures code-spec context is injected to agents, resulting in higher quality code.
> The overhead is minimal, but the benefit is significant.

---

## Question / Trivial Fix

For questions or trivial fixes, work directly:

1. Answer question or make the fix
2. If code was changed, remind user to run `/`

---

## Simple Task

For simple, well-defined tasks:

1. Quick confirm: "I understand you want to [goal]. Shall I proceed?"
2. If no, clarify and confirm again
3. **If yes: execute ALL steps below without stopping. Do NOT ask for additional confirmation between steps.**
   - Create task directory (Phase 1 Path B, Step 2)
   - Write PRD (Step 3)
   - Research codebase (Phase 2, Step 5)
   - Configure context (Step 6)
   - Activate task (Step 7)
   - Implement (Phase 3, Step 8)
   - Check quality (Step 9)
   - Complete (Step 10)

---

## Complex Task - Brainstorm First

For complex or vague tasks, **automatically start the brainstorm process** �?do NOT skip directly to implementation.

See `/` for the full process. Summary:

1. **Acknowledge and classify** - State your understanding
2. **Create task directory** - Track evolving requirements in `prd.md`
3. **Ask questions one at a time** - Update PRD after each answer
4. **Propose approaches** - For architectural decisions
5. **Confirm final requirements** - Get explicit approval
6. **Proceed to Task Workflow** - With clear requirements in PRD

> **Subtask Decomposition**: If brainstorm reveals multiple independent work items,
> consider creating subtasks using `--parent` flag or `add-subtask` command.
> See `/` Step 8 for details.

### Key Brainstorm Principles

| Principle | Description |
|-----------|-------------|
| **One question at a time** | Never overwhelm with multiple questions |
| **Update PRD immediately** | After each answer, update the document |
| **Prefer multiple choice** | Easier for users to answer |
| **YAGNI** | Challenge unnecessary complexity |

---

## Task Workflow (Development Tasks)

**Why this workflow?**
- Research Agent analyzes what code-spec files are needed
- Code-spec files are configured in jsonl files
- Implement Agent receives code-spec context via Hook injection
- Check Agent verifies against code-spec requirements
- Result: Code that follows project conventions automatically

### Overview: Two Entry Points

```
From Brainstorm (Complex Task):
  PRD confirmed �?Research �?Configure Context �?Activate �?Implement �?Check �?Complete

From Simple Task:
  Confirm �?Create Task �?Write PRD �?Research �?Configure Context �?Activate �?Implement �?Check �?Complete
```

**Key principle: Research happens AFTER requirements are clear (PRD exists).**

---

### Phase 1: Establish Requirements

#### Path A: From Brainstorm (skip to Phase 2)

PRD and task directory already exist from brainstorm. Skip directly to Phase 2.

#### Path B: From Simple Task

**Step 1: Confirm Understanding** `[AI]`

Quick confirm:
- What is the goal?
- What type of development? (frontend / backend / fullstack)
- Any specific requirements or constraints?

**Step 2: Create Task Directory** `[AI]`

```bash
TASK_DIR=$(python3 ./.harness-cli/scripts/task.py create "<title>" --slug <name>)
```

**Step 3: Write PRD** `[AI]`

Create `prd.md` in the task directory with:

```markdown
# <Task Title>

## Goal
<What we're trying to achieve>

## Requirements
- <Requirement 1>
- <Requirement 2>

## Acceptance Criteria
- [ ] <Criterion 1>
- [ ] <Criterion 2>

## Technical Notes
<Any technical decisions or constraints>
```

---

### Phase 2: Prepare for Implementation (shared)

> Both paths converge here. PRD and task directory must exist before proceeding.

**Step 4: Code-Spec Depth Check** `[AI]`

If the task touches infra or cross-layer contracts, do not start implementation until code-spec depth is defined.

Trigger this requirement when the change includes any of:
- New or changed command/API signatures
- Database schema or migration changes
- Infra integrations (storage, queue, cache, secrets, env contracts)
- Cross-layer payload transformations

Must-have before proceeding:
- [ ] Target code-spec files to update are identified
- [ ] Concrete contract is defined (signature, fields, env keys)
- [ ] Validation and error matrix is defined
- [ ] At least one Good/Base/Bad case is defined

**Step 5: Research the Codebase** `[AI]`

Based on the confirmed PRD, call Research Agent to find relevant specs and patterns:

```
Task(
  subagent_type: "research",
  prompt: "Analyze the codebase for this task:

  Task: <goal from PRD>
  Type: <frontend/backend/fullstack>

  Please find:
  1. Relevant code-spec files in .harness-cli/spec/
  2. Existing code patterns to follow (find 2-3 examples)
  3. Files that will likely need modification

  Output:
  ## Relevant Code-Specs
  - <path>: <why it's relevant>

  ## Code Patterns Found
  - <pattern>: <example file path>

  ## Files to Modify
  - <path>: <what change>",
  model: "opus"
)
```

**Step 6: Configure Context** `[AI]`

Initialize default context:

```bash
python3 ./.harness-cli/scripts/task.py init-context "$TASK_DIR" <type>
# type: backend | frontend | fullstack
```

Add code-spec files found by Research Agent:

```bash
# For each relevant code-spec and code pattern:
python3 ./.harness-cli/scripts/task.py add-context "$TASK_DIR" implement "<path>" "<reason>"
python3 ./.harness-cli/scripts/task.py add-context "$TASK_DIR" check "<path>" "<reason>"
```

**Step 7: Activate Task** `[AI]`

```bash
python3 ./.harness-cli/scripts/task.py start "$TASK_DIR"
```

This sets `.current-task` so hooks can inject context.

---

### Phase 3: Execute (shared)

**Step 8: Implement** `[AI]`

Call Implement Agent (code-spec context is auto-injected by hook):

```
Task(
  subagent_type: "implement",
  prompt: "Implement the task described in prd.md.

  Follow all code-spec files that have been injected into your context.
  Run lint and typecheck before finishing.",
  model: "opus"
)
```

**Step 9: Check Quality** `[AI]`

Call Check Agent (code-spec context is auto-injected by hook):

```
Task(
  subagent_type: "check",
  prompt: "Review all code changes against the code-spec requirements.

  Fix any issues you find directly.
  Ensure lint and typecheck pass.",
  model: "opus"
)
```

**Step 10: Complete** `[AI]`

1. Verify lint and typecheck pass
2. Report what was implemented
3. Remind user to:
   - Test the changes
   - Commit when ready
   - Run `/` to record this session

---

## Continuing Existing Task

If `get_context.py` shows a current task:

1. Read the task's `prd.md` to understand the goal
2. Check `task.json` for current status and phase
3. Ask user: "Continue working on <task-name>?"

If yes, resume from the appropriate step (usually Step 7 or 8).

---

## Commands Reference

### User Commands `[USER]`

| Command | When to Use |
|---------|-------------|
| `/` | Begin a dev session (default mode) |
| `start debug [desc]` | Begin a debug diagnosis session |
| `start arch [module]` | Begin an architecture review session |
| `/` | Clarify vague requirements (called from start) |
| `/` | Complex tasks needing isolated worktree |
| `/` | Before committing changes |
| `/` | After completing a task |

### AI Scripts `[AI]`

| Script | Purpose |
|--------|---------|
| `python3 ./.harness-cli/scripts/get_context.py` | Get session context |
| `python3 ./.harness-cli/scripts/task.py create` | Create task directory |
| `python3 ./.harness-cli/scripts/task.py init-context` | Initialize jsonl files |
| `python3 ./.harness-cli/scripts/task.py add-context` | Add code-spec/context file to jsonl |
| `python3 ./.harness-cli/scripts/task.py start` | Set current task |
| `python3 ./.harness-cli/scripts/task.py finish` | Clear current task |
| `python3 ./.harness-cli/scripts/task.py archive` | Archive completed task |

### Sub Agents `[AI]`

| Agent | Purpose | Hook Injection |
|-------|---------|----------------|
| research | Analyze codebase | No (reads directly) |
| implement | Write code | Yes (implement.jsonl) |
| check | Review & fix | Yes (check.jsonl) |
| debug | Fix specific issues | Yes (debug.jsonl) |

---

## Debug Mode (Structured Diagnosis)

A 7-step structured process for bug diagnosis and resolution. Entered via `start debug [description]`.

### Debug Step 1: Collect Information `[AI]`

Gather diagnostic information from the user **one question at a time**. Do NOT ask multiple questions at once.

**Information to collect (in order):**

1. **Symptom description** — What is happening? What was expected?
2. **Error logs** — Ask for relevant error messages or stack traces
3. **Reproduction steps** — How to trigger the issue
4. **Environment info** — OS, version, configuration, recent changes

> **Shortcut**: If user provided a description with `start debug "..."`, use it as the symptom description and skip to the next missing piece.

After each answer, summarize what you know so far before asking the next question.

### Debug Step 2: Create Debug Task `[AI]`

Once you have enough context (at minimum: symptom + error log or reproduction steps):

```bash
TASK_DIR=$(python3 ./.harness-cli/scripts/task.py create "debug: <short description>" --slug debug-<name>)
```

Write a `prd.md` in the task directory:

```markdown
# Debug: <Short Description>

## Symptom
<What is happening>

## Expected Behavior
<What should happen>

## Error Logs
<Paste error logs>

## Reproduction Steps
<Steps to reproduce>

## Environment
<OS, version, config>
```

Activate the task:

```bash
python3 ./.harness-cli/scripts/task.py start "$TASK_DIR"
```

### Debug Step 3: Automatic Scan `[AI]`

Based on the collected information, perform automated investigation:

1. **Keyword search** — Search codebase for keywords from error messages:
   - Error class/type names
   - Function names in stack traces
   - Error message text

2. **Recent changes** — Check git history for relevant recent changes:
   ```bash
   git log --oneline -20
   git log --oneline -10 -- <relevant-files>
   ```

3. **Related code** — Read the source files identified from logs and search results

### Debug Step 4: Form Hypotheses `[AI]`

Based on the scan results, present 2-3 hypotheses to the user:

```markdown
## Hypotheses (ranked by likelihood)

### H1: <Most likely cause> (High)
- **Evidence**: <what points to this>
- **Verification**: <how to confirm>

### H2: <Second possibility> (Medium)
- **Evidence**: <what points to this>
- **Verification**: <how to confirm>

### H3: <Less likely> (Low)
- **Evidence**: <what points to this>
- **Verification**: <how to confirm>
```

Ask user: "Does this match your intuition? Should I start verifying from H1?"

### Debug Step 5: Verify Hypotheses `[AI]`

For each hypothesis (in priority order):

1. **Read relevant code** — Trace the execution path
2. **Check edge cases** — Look for missing null checks, race conditions, etc.
3. **Write a test to reproduce** — If possible, create a minimal test case
4. **Report findings** — Confirm or rule out the hypothesis

If a hypothesis is confirmed, proceed to Step 6. If ruled out, move to the next hypothesis.

### Debug Step 6: Implement Fix `[AI]`

Once root cause is confirmed:

1. **Configure context** for the fix:
   ```bash
   python3 ./.harness-cli/scripts/task.py init-context "$TASK_DIR" <type>
   ```

2. **Call Implement Agent** to apply the fix:
   ```
   Task(
     subagent_type: "implement",
     prompt: "Implement the fix described in prd.md.

     Root cause: <confirmed root cause>
     Fix approach: <what to change>

     Follow all code-spec files that have been injected into your context.
     Run lint and typecheck before finishing.",
     model: "opus"
   )
   ```

3. **Call Check Agent** to verify:
   ```
   Task(
     subagent_type: "check",
     prompt: "Review the debug fix against the code-spec requirements.

     Fix any issues you find directly.
     Ensure lint and typecheck pass.",
     model: "opus"
   )
   ```

### Debug Step 7: Knowledge Capture `[AI]`

After the fix is verified, perform knowledge capture:

1. **Trigger break-loop analysis** — Run the break-loop analysis framework to:
   - Categorize the root cause (Missing Spec / Cross-Layer Contract / Change Propagation / Test Gap / Implicit Assumption)
   - Identify prevention mechanisms
   - Check for similar issues elsewhere

2. **Update specs if needed** — If the bug was caused by missing or incomplete documentation:
   - Update relevant spec files in `.harness-cli/spec/`
   - Add guard rails or checklists to prevent recurrence

3. **Complete the task**:
   ```bash
   python3 ./.harness-cli/scripts/task.py finish
   ```

4. Remind user to:
   - Test the fix
   - Commit when ready
   - Record this session

---

## Arch Mode (Architecture Review)

A structured architecture review process that scans the codebase, generates analysis, and optionally creates improvement tasks. Entered via `start arch [module]`.

### Arch Step 1: Determine Scope `[AI]`

Determine the review scope based on user input:

| Input | Scope |
|-------|-------|
| `start arch` | Full project scan |
| `start arch <module>` | Scoped to specified module/package |

If scoped, confirm the module path exists and clarify boundaries.

### Arch Step 2: Automatic Scan `[AI]`

Scan the codebase across 5 dimensions:

#### 2a. Code Structure
- Module organization and directory layout
- Responsibility boundaries (is each module focused?)
- Dependency direction (do lower layers depend on higher layers?)

#### 2b. Spec Coverage
- Which modules have spec files in `.harness-cli/spec/`?
- Which modules are missing specs?
- Are existing specs up to date with the code?

Read spec index files:
```bash
python3 ./.harness-cli/scripts/get_context.py --mode packages
```

#### 2c. Code Smells
- Duplicated logic across files
- Functions that are too long (> 100 lines)
- Deep nesting (> 3 levels)
- Files that do too many things

#### 2d. Consistency
- Error handling patterns (consistent across modules?)
- Naming conventions (files, functions, variables)
- API design patterns (input/output formats)

#### 2e. Extensibility
- How much code changes when adding a new feature?
- Are there clear extension points?
- Is configuration separated from logic?

### Arch Step 3: Generate Report `[AI]`

Present the analysis as a structured report:

```markdown
## Architecture Analysis Report

### Scope
<Full project / Module: xxx>

### Summary
<1-2 paragraph overview of architecture health>

### Findings (by priority)

#### P0 - Critical
- **[Finding title]**: <description>
  - Impact: <what breaks or is blocked>
  - Suggestion: <how to fix>

#### P1 - Important
- **[Finding title]**: <description>
  - Impact: <what's affected>
  - Suggestion: <how to improve>

#### P2 - Nice to Have
- **[Finding title]**: <description>
  - Suggestion: <improvement idea>

### Dimension Scores
| Dimension | Score | Notes |
|-----------|-------|-------|
| Code Structure | Good/Fair/Poor | ... |
| Spec Coverage | Good/Fair/Poor | ... |
| Code Smells | Good/Fair/Poor | ... |
| Consistency | Good/Fair/Poor | ... |
| Extensibility | Good/Fair/Poor | ... |

### Positive Patterns
<Things the codebase does well — acknowledge good practices>
```

### Arch Step 4: Discussion `[AI]`

Present the report and ask for user feedback:

- "Here is the architecture analysis. Do you have questions about any findings?"
- "Are there specific areas you'd like me to dig deeper into?"
- Adjust analysis based on user input

### Arch Step 5: Optional Task Decomposition `[AI]`

After the user has reviewed the report, ask:

> "Would you like me to create development tasks from these findings? I can create individual tasks for each actionable item."

**If yes:**

For each actionable finding, create a task:

```bash
TASK_DIR=$(python3 ./.harness-cli/scripts/task.py create "arch: <finding title>" --slug arch-<name>)
```

Write a `prd.md` in each task directory with:
- The finding description
- Suggested approach
- Acceptance criteria
- Reference to the arch analysis

After creating tasks, list them:
```bash
python3 ./.harness-cli/scripts/task.py list
```

**If no:** End the arch review session. Remind user they can revisit the findings later.

---

## Mode Switching

### Dev to Debug

If during dev mode you discover the user's task is actually a bug:

1. Inform the user: "This looks like a bug rather than a feature request. Switch to debug mode?"
2. If confirmed, transition to **Debug Step 1** (carry over any context already gathered)
3. The existing task (if any) remains active — create a new debug task

### Debug to Dev

After completing Debug Step 7 (knowledge capture), if there are remaining dev tasks:

1. The debug task is completed
2. Resume the previous dev session or ask: "The bug is fixed. Continue with development?"

### Arch to Dev

When executing tasks created from an arch review:

1. Each arch task enters dev mode via the normal Task Workflow
2. The arch task's PRD already contains requirements and approach

---

## Key Principle

> **Code-spec context is injected, not remembered.**
>
> The Task Workflow ensures agents receive relevant code-spec context automatically.
> This is more reliable than hoping the AI "remembers" conventions.
