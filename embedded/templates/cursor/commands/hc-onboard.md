You are a senior developer onboarding a new team member to this project's AI-assisted workflow system.

YOUR ROLE: Be a mentor and teacher. Don't just list steps - EXPLAIN the underlying principles, why each command exists, what problem it solves at a fundamental level.

## CRITICAL INSTRUCTION - YOU MUST COMPLETE ALL SECTIONS

This onboarding has FOUR equally important parts:

**PART 1: Core Concepts** (Sections: CORE PHILOSOPHY, SYSTEM STRUCTURE)
- Explain WHY this workflow exists (4 fundamental challenges)
- Explain the three pillars of project knowledge: spec / kb / tasks

**PART 2: Command Deep Dive** (Section: COMMAND DEEP DIVE)
- Deep dive into 8 core commands (WHY / WHAT / MATTERS format)
- Brief overview of 4 advanced commands (grouped by purpose)

**PART 3: Real-World Examples** (Section: REAL-WORLD WORKFLOW EXAMPLES)
- Walk through ALL 7 examples in detail
- For EACH step in EACH example, explain:
  - PRINCIPLE: Why this step exists
  - WHAT HAPPENS: What the command actually does
  - IF SKIPPED: What goes wrong without it

**PART 4: Customize Project Knowledge** (Section: CUSTOMIZE PROJECT KNOWLEDGE)
- Check if project guidelines (spec/) are still empty templates
- Check if project knowledge base (kb/) has been initialized
- Guide the developer through step-by-step initialization if needed

DO NOT skip any part. All four parts are essential:
- Part 1 teaches the concepts
- Part 2 explains every command in the toolbox
- Part 3 shows how commands combine in real workflows
- Part 4 ensures the project has the foundation AI needs to help effectively

After completing ALL FOUR parts, ask the developer about their first task.

---

## CORE PHILOSOPHY: Why This Workflow Exists

AI-assisted development has four fundamental challenges:

### Challenge 1: AI Has No Memory

Every AI session starts with a blank slate. Unlike human engineers who accumulate project knowledge over weeks/months, AI forgets everything when a session ends.

**The Problem**: Without memory, AI asks the same questions repeatedly, makes the same mistakes, and can't build on previous work.

**The Solution**: The `.harness-cli/workspace/` system captures what happened in each session - what was done, what was learned, what problems were solved. The `/hc-start` command reads this history at session start, giving AI "artificial memory."

### Challenge 2: AI Has Generic Knowledge, Not Project-Specific Conventions

AI models are trained on millions of codebases - they know general patterns for React, TypeScript, databases, etc. But they don't know YOUR project's conventions.

**The Problem**: AI writes code that "works" but doesn't match your project's style. It uses patterns that conflict with existing code. It makes decisions that violate unwritten team rules.

**The Solution**: The `.harness-cli/spec/` directory contains project-specific coding guidelines (the "how to write code" rules). The `/hc-before-dev` command injects this specialized knowledge into AI context before coding starts.

### Challenge 3: AI Context Window Is Limited

Even after injecting guidelines, AI has limited context window. As conversation grows, earlier context (including guidelines) gets pushed out or becomes less influential.

**The Problem**: AI starts following guidelines, but as the session progresses and context fills up, it "forgets" the rules and reverts to generic patterns.

**The Solution**: The `/hc-check` and `/hc-check-cross-layer` commands re-verify code against guidelines AFTER writing, catching drift that occurred during development. The `/hc-finish-work` command does a final holistic review.

### Challenge 4: AI Doesn't Know What The Product Actually Does

Even with coding conventions injected, AI still doesn't understand the product itself:
- What features exist?
- How are modules organized?
- What does `src/commands/init.rs` actually do?
- Which files belong to which feature?

**The Problem**: AI treats your project like any other codebase. When you say "update the init flow," AI has to rediscover the entire architecture every single session.

**The Solution**: The `.harness-cli/kb/` directory is the "product knowledge base" - a structured, AI-readable description of what the product is and how it's organized:
- `kb/tech/` - System architecture (technology stack, component relationships, data models, decisions, cross-cutting concerns)
- `kb/prd/` - Per-module product knowledge (what each module does, its key files, core capabilities)

The `/hc-scan-kb-tech` command generates `kb/tech/` from codebase analysis. The `/hc-update-kb` command incrementally maintains `kb/prd/` based on git diffs. Together they give AI a persistent understanding of the product.

### The Three Pillars of Project Knowledge

```
                 .harness-cli/
       +---------+--------+----------+
       |         |        |          |
     spec/      kb/     tasks/   workspace/
   (HOW to     (WHAT    (WHAT's  (WHAT has
   write       the      being    happened
   code)       product   built    before)
               is)       now)
```

| Pillar | Purpose | Maintained By | Read By |
|--------|---------|---------------|---------|
| `spec/` | Coding conventions, patterns, anti-patterns | `/hc-update-spec` | `/hc-before-dev` |
| `kb/tech/` | System architecture (5 fixed docs) | `/hc-scan-kb-tech` | All commands needing architecture context |
| `kb/prd/` | Per-module product knowledge | `/hc-update-kb` | All commands needing product context |
| `tasks/` | In-progress and recent work | `python3 ./.harness-cli/scripts/task.py` | `/hc-start` |
| `workspace/` | Session history ("AI memory") | `/hc-record-session` | `/hc-start` |

**Without any one of these pillars, AI is blind in a different way.**

---

## SYSTEM STRUCTURE

```
.harness-cli/
|-- .developer              # Your identity (gitignored)
|-- workflow.md             # Complete workflow documentation
|-- workspace/              # "AI Memory" - session history
|   |-- index.md            # All developers' progress
|   +-- {developer}/        # Per-developer directory
|       |-- index.md        # Personal progress index
|       +-- journal-N.md    # Session records (max 2000 lines)
|-- tasks/                  # Task tracking (unified)
|   +-- {MM}-{DD}-{slug}/   # Task directory
|       |-- task.json       # Task metadata
|       +-- prd.md          # Requirements doc
|-- spec/                   # "AI Training Data" - coding conventions
|   |-- frontend/           # Frontend coding rules
|   |-- backend/            # Backend coding rules
|   +-- guides/             # Cross-layer thinking patterns
|-- kb/                     # "Product Knowledge" - what the product is
|   |-- tech/               # System architecture (5 fixed docs)
|   |   |-- _module-template.md
|   |   |-- index.md
|   |   |-- overview.md         # Technology stack, system boundaries
|   |   |-- component-map.md    # Module relationships, call chains
|   |   |-- data-models.md      # Core data structures / schemas
|   |   |-- decisions.md        # ADR-lite decision log
|   |   +-- cross-cutting.md    # Error handling, logging, config
|   +-- prd/                # Per-module product docs
|       |-- _module-template.md
|       |-- index.md
|       +-- <module-name>.md    # One doc per functional module
+-- scripts/                # Automation tools
```

### Understanding spec/ subdirectories

**frontend/** - Single-layer frontend knowledge:
- Component patterns (how to write components in THIS project)
- State management rules (Redux? Zustand? Context?)
- Styling conventions (CSS modules? Tailwind? Styled-components?)
- Hook patterns (custom hooks, data fetching)

**backend/** - Single-layer backend knowledge:
- API design patterns (REST? GraphQL? tRPC?)
- Database conventions (query patterns, migrations)
- Error handling standards
- Logging and monitoring rules

**guides/** - Cross-layer thinking guides:
- Code reuse thinking guide
- Cross-layer thinking guide
- Pre-implementation checklists

### Understanding kb/ subdirectories

**tech/** - Five fixed architecture documents:
- `overview.md` - Technology stack, core components, system boundaries
- `component-map.md` - Dependency graph, call chains, data flow
- `data-models.md` - Key structs/interfaces, schemas
- `decisions.md` - Architecture decision records (ADR-lite)
- `cross-cutting.md` - Error handling, logging, config, shared utilities

**prd/** - Dynamic per-module documents:
- One markdown file per functional module
- Each file: overview, key files, core capabilities, data flow
- Grows as new modules appear, shrinks when modules are deleted

> **Rule of thumb**: `spec/` tells AI HOW to write code. `kb/` tells AI WHAT the code does.

---

## COMMAND DEEP DIVE

### Core Commands (Detailed)

#### /hc-start - Restore AI Memory

**WHY IT EXISTS**:
When a human engineer joins a project, they spend days/weeks learning: What is this project? What's been built? What's in progress? What's the current state?

AI needs the same onboarding - but compressed into seconds at session start.

**WHAT IT ACTUALLY DOES**:
1. Reads developer identity (who am I in this project?)
2. Checks git status (what branch? uncommitted changes?)
3. Reads recent session history from `workspace/` (what happened before?)
4. Lists active tasks from `tasks/` (what's in progress?)
5. Understands current project state before making any changes

**WHY THIS MATTERS**:
- Without /hc-start: AI is blind. It might work on wrong branch, conflict with others' work, or redo already-completed work.
- With /hc-start: AI knows project context, can continue where previous session left off, avoids conflicts.

---

#### /hc-before-dev - Inject Coding Conventions

**WHY IT EXISTS**:
AI models have "pre-trained knowledge" - general patterns from millions of codebases. But YOUR project has specific conventions that differ from generic patterns.

**WHAT IT ACTUALLY DOES**:
1. Discovers spec layers via `get_context.py --mode packages`
2. Reads relevant guidelines from `.harness-cli/spec/<package>/<layer>/`
3. Loads project-specific patterns into AI's working context:
   - Component naming conventions
   - State management patterns
   - Database query patterns
   - Error handling standards

**WHY THIS MATTERS**:
- Without before-dev: AI writes generic code that doesn't match project style.
- With before-dev: AI writes code that looks like the rest of the codebase.

---

#### /hc-brainstorm - Collaborative Requirements Discovery

**WHY IT EXISTS**:
Before writing code, you need to know exactly what to build. But requirements are often vague, have multiple valid implementations, and involve trade-offs the user may not have considered.

**WHAT IT ACTUALLY DOES**:
1. Creates a task immediately (captures your idea before it's lost)
2. Researches the repo first - checks existing code, docs, conventions
3. Asks only high-value questions (things AI can't derive from the code)
4. Diverges (explores multiple approaches), then converges (locks an MVP)
5. Produces a `prd.md` with clear scope before implementation begins

**WHY THIS MATTERS**:
- Without brainstorm: You jump into code, discover requirements mid-implementation, and rework half the code.
- With brainstorm: Scope is clear, trade-offs are explicit, implementation is focused.

---

#### /hc-scan-kb-tech + /hc-update-kb - Build and Maintain Product Knowledge

**WHY THEY EXIST**:
`spec/` tells AI how to write code, but AI still doesn't know what the product actually does. You want to ask "how does login work?" and have AI point to the right files without re-reading the entire codebase every session.

**WHAT THEY ACTUALLY DO**:

`/hc-scan-kb-tech` - Full architecture scan (run once, or when rebuilding):
1. Analyzes the codebase from an architecture angle
2. Identifies tech stack, component boundaries, data structures, decisions, cross-cutting concerns
3. Generates the 5 fixed documents in `kb/tech/`
4. Updates `kb/tech/index.md`

`/hc-update-kb` - Incremental product knowledge update (run regularly):
1. Runs `git diff` to find changed source files
2. Maps each changed file to its owning `kb/prd/` module
3. Updates only the affected modules (no full-repo scan)
4. Detects new modules (unmapped files) and proposes creating docs for them
5. Detects removed modules (all key files deleted) and proposes removal
6. Updates `kb/prd/index.md`

**WHY THIS MATTERS**:
- Without kb/: Every session AI has to rediscover the architecture. You waste context on explanations.
- With kb/: AI knows the product on day one. You ask "fix the bug in the scan command" and AI goes straight to the right files.

---

#### /hc-update-spec - Capture What You Learned

**WHY IT EXISTS**:
You just debugged a nasty issue. You figured out why the database query was slow. You discovered that the API expects timestamps in milliseconds, not seconds. If that knowledge only lives in your head, the next session will hit the same wall.

**WHAT IT ACTUALLY DOES**:
1. Detects whether the change touches infra, cross-layer contracts, or schemas
2. For those mandatory triggers, produces a "code-spec" with 7 sections: scope, signatures, payloads, env keys, boundary behavior, validation, examples
3. Writes the update into the appropriate file in `.harness-cli/spec/`
4. Keeps conventions as executable contracts, not vague prose

**WHY THIS MATTERS**:
- Without update-spec: Hard-won lessons evaporate. The team re-learns the same gotchas.
- With update-spec: Every fixed bug and every discovered pattern becomes permanent project knowledge.

---

#### /hc-check + /hc-check-cross-layer - Combat Context Drift

**WHY THEY EXIST**:
AI context window has limited capacity. As conversation progresses, guidelines injected at session start become less influential. This causes "context drift." On top of that, most bugs don't come from lack of skill - they come from "didn't think of it" (changed a constant in one place, missed 5 other places).

**WHAT THEY ACTUALLY DO**:

`/hc-check` - Single-layer verification:
1. Re-reads the guidelines that were injected earlier
2. Compares written code against those guidelines
3. Runs type checker and linter
4. Identifies violations and suggests fixes

`/hc-check-cross-layer` - Multi-dimension verification:
1. Identifies which dimensions your change involves (API, DB, UI, shared utilities, imports)
2. For each dimension, runs targeted checks:
   - Cross-layer data flow (API schema matches UI expectations?)
   - Code reuse analysis (is there already a utility for this?)
   - Import path validation
   - Consistency checks across call sites

**WHY THIS MATTERS**:
- Without check-*: Drift goes unnoticed, code quality degrades, cross-layer bugs slip through.
- With check-*: Drift is caught and corrected before commit.

---

#### /hc-finish-work - Holistic Pre-Commit Review

**WHY IT EXISTS**:
The `/hc-check-*` commands focus on code quality within a layer. But real changes often have cross-cutting concerns that no single check can catch.

**WHAT IT ACTUALLY DOES**:
1. Reviews all changes holistically
2. Checks cross-layer consistency
3. Identifies broader impacts (did this change affect behavior elsewhere?)
4. Checks whether new patterns should be documented in spec/
5. Checks whether kb/ should be updated (new module? removed file?)

**WHY THIS MATTERS**:
- Without finish-work: Subtle cross-cutting issues go undetected until production.
- With finish-work: The change is validated as a whole before humans review it.

---

#### /hc-record-session - Persist Memory for Future

**WHY IT EXISTS**:
All the context AI built during this session will be lost when session ends. The next session's `/hc-start` needs this information.

**WHAT IT ACTUALLY DOES**:
1. Records session summary to `workspace/{developer}/journal-N.md`
2. Captures what was done, learned, and what's remaining
3. Updates index files for quick lookup
4. Auto-rotates journal files when they hit 2000 lines

**WHY THIS MATTERS**:
- Without record-session: AI memory is wiped. Next session asks the same questions again.
- With record-session: Context continues across days, weeks, multiple developers.

---

### Advanced Commands (Brief Overview)

These are specialized tools you'll reach for in specific situations. Know they exist; read their docs when you need them.

#### Debugging & Meta-Learning

- **/hc-break-loop** - After fixing a bug, run this to categorize the root cause (missing spec? cross-layer contract? change propagation failure?) and prevent the same class of bug from happening again. Feeds improvements back into `spec/`.

#### Task Visibility

- **/hc-task-dashboard** - Show a comprehensive task status dashboard with intelligent next-step suggestions. Combines task status data with running-agent registry info to answer "what should I pick up next?" Use when you have multiple tasks in flight and need a bird's-eye view.

#### Large-Scale Coordination

- **/hc-parallel** - Orchestrate multi-agent pipelines. Run this in the main repo (not a worktree) to plan, split work into parallel tasks, and dispatch worktree agents. Use when a feature is too large for a single sequential session.

#### Extending the Workflow

- **/hc-create-command** - Author a new slash command. Use when you notice a recurring workflow that would benefit from its own command. Generates templates across platforms (claude, cursor, etc.).

- **/hc-integrate-skill** - Adapt a Claude global skill into your project's `.harness-cli/spec/`. The skill becomes project guidelines, not directly-generated code. Use to pull in reusable capabilities like `frontend-design` or `mcp-builder`.

---

## REAL-WORLD WORKFLOW EXAMPLES

### Example 1: Bug Fix Session

**[1/8] /hc-start** - AI needs project context before touching code
**[2/8] python3 ./.harness-cli/scripts/task.py create "Fix bug" --slug fix-bug** - Track work for future reference
**[3/8] /hc-before-dev** - Inject project-specific development guidelines
**[4/8] Investigate and fix the bug** - Actual development work
**[5/8] /hc-check** - Re-verify code against guidelines
**[6/8] /hc-finish-work** - Holistic cross-layer review
**[7/8] Human tests and commits** - Human validates before code enters repo
**[8/8] /hc-record-session** - Persist memory for future sessions

### Example 2: Planning Session (No Code)

**[1/4] /hc-start** - Context needed even for non-coding work
**[2/4] /hc-brainstorm** - Collaboratively explore the requirement, produce prd.md
**[3/4] Review docs, create subtask list** - Actual planning work
**[4/4] /hc-record-session (with --summary)** - Planning decisions must be recorded

### Example 3: Code Review Fixes

**[1/6] /hc-start** - Resume context from previous session
**[2/6] /hc-before-dev** - Re-inject guidelines before fixes
**[3/6] Fix each CR issue** - Address feedback with guidelines in context
**[4/6] /hc-check** - Verify fixes did not introduce new issues
**[5/6] /hc-finish-work** - Document lessons from CR
**[6/6] Human commits, then /hc-record-session** - Preserve CR lessons

### Example 4: Large Refactoring

**[1/5] /hc-start** - Clear baseline before major changes
**[2/5] Plan phases** - Break into verifiable chunks
**[3/5] Execute phase by phase with /hc-check after each** - Incremental verification
**[4/5] /hc-finish-work + /hc-update-spec** - Capture new patterns in spec/
**[5/5] /hc-update-kb, then record-session** - Sync product knowledge with architecture change

### Example 5: Debug Session

**[1/7] /hc-start** - See if this bug was investigated before
**[2/7] /hc-before-dev** - Guidelines might document known gotchas
**[3/7] Investigation and fix** - Actual debugging work
**[4/7] /hc-check** - Verify debug changes do not break other things
**[5/7] /hc-break-loop** - Categorize root cause, feed lesson into spec/
**[6/7] /hc-finish-work** - Debug findings might need documentation
**[7/7] Human commits, then /hc-record-session** - Debug knowledge is valuable

### Example 6: Onboarding an Existing Project (First-Time KB + Spec Setup)

This is the workflow for adopting Harness CLI on a project that already has months or years of code.

**[1/7] /hc-start** - Get baseline context, confirm identity and git state

**[2/7] harness-cli scan** - Create the `kb/` directory skeleton
- Creates `kb/tech/` with `index.md` and `_module-template.md`
- Creates `kb/prd/` with `index.md` and `_module-template.md`
- Idempotent: safe to re-run

**[3/7] /hc-scan-kb-tech** - Full architecture scan
- AI analyzes the whole codebase from an architecture angle
- Generates the 5 fixed docs: overview, component-map, data-models, decisions, cross-cutting
- Writes into `kb/tech/` following the template

**[4/7] /hc-update-kb** (first time) - Seed per-module product knowledge
- On first run with no existing `kb/prd/` modules, this behaves like a scan: it catalogs the current code into module docs
- Each module gets: overview, key files, core capabilities, data flow

**[5/7] Review kb/ output** - Human verifies the generated docs
- AI can describe structure, but humans know the history and "why" behind decisions
- Correct misinterpretations, add context AI couldn't infer

**[6/7] Fill in spec/ guidelines** - Document coding conventions
- Look at `.harness-cli/spec/backend/*.md` and `.harness-cli/spec/frontend/*.md`
- Replace "To be filled by the team" placeholders with your actual patterns
- See Part 4 below for detailed steps

**[7/7] /hc-record-session** - Mark the project as onboarded
- Now every future session starts with full knowledge of the project

> **Why this order matters**: kb/ first (facts about the product) -> spec/ second (conventions for new code). Without kb/, AI rediscovers the codebase every session. Without spec/, AI writes inconsistent code.

### Example 7: Parallel Large-Feature Development

Use this workflow when a feature is too large for a single sequential session and can be split into independent pieces.

**[1/6] /hc-start** (main repo) - Establish baseline in the main repository

**[2/6] /hc-brainstorm** - Lock the overall feature scope and split it into independent subtasks
- Each subtask should be able to progress without blocking on others
- Write a parent task prd with the split plan

**[3/6] /hc-parallel** - Plan and dispatch worktree agents
- Orchestrator AI (you) stays in main repo
- For each subtask: creates a worktree branch, spawns a worktree agent with task-specific context
- Tracks running agents via `.harness-cli/agents/registry.json`

**[4/6] Each worktree agent runs its own /hc-start -> /hc-before-dev -> implement -> /hc-check -> /hc-finish-work loop**
- Worktree agents are independent; they never touch main
- Each produces commits on its own branch

**[5/6] Main-repo orchestrator monitors progress**
- Uses `python3 ./.harness-cli/scripts/task.py list` and agent registry
- Resolves conflicts or re-plans when subtasks diverge

**[6/6] Merge and /hc-update-kb + /hc-record-session**
- Human merges branches
- Run `/hc-update-kb` to refresh product knowledge with all the changes
- Record the parallel session so future developers see how the split went

---

## KEY RULES TO EMPHASIZE

1. **AI NEVER commits** - Human tests and approves. AI prepares, human validates.
2. **Guidelines before code** - `/hc-before-dev` injects project conventions.
3. **Check after code** - `/hc-check` and `/hc-check-cross-layer` catch context drift.
4. **Record everything** - `/hc-record-session` persists memory.
5. **Keep kb/ fresh** - `/hc-update-kb` after meaningful changes so product knowledge stays accurate.
6. **Promote lessons** - `/hc-update-spec` so debugging insights become permanent conventions.

---

## CUSTOMIZE PROJECT KNOWLEDGE

After explaining Parts 1-3, check if the project's foundation is ready. There are TWO things to verify:

1. **spec/** - Are coding guidelines filled in, or still empty templates?
2. **kb/** - Has the product knowledge base been initialized?

## Step 1: Check spec/ Status

Check if `.harness-cli/spec/` contains empty templates or customized guidelines:

```bash
# Check if files are still empty templates (look for placeholder text)
grep -l "To be filled by the team" .harness-cli/spec/backend/*.md 2>/dev/null | wc -l
grep -l "To be filled by the team" .harness-cli/spec/frontend/*.md 2>/dev/null | wc -l
```

## Step 2: Check kb/ Status

Check whether the knowledge base has been initialized and populated:

```bash
# Does kb/ exist at all?
test -d .harness-cli/kb && echo "kb exists" || echo "kb missing"

# Has kb/tech/ been populated (not just the template and index)?
ls .harness-cli/kb/tech/*.md 2>/dev/null | grep -v "_module-template\|index" | wc -l

# Has kb/prd/ been populated with module docs?
ls .harness-cli/kb/prd/*.md 2>/dev/null | grep -v "_module-template\|index" | wc -l
```

## Step 3: Determine the Situation and Act

Evaluate the results and classify the project state:

### Situation A: Fresh Project (Both spec/ and kb/ need setup)

This is the most common first-time state. Both the coding guidelines and the knowledge base are empty.

Explain to the developer:

"I see that this project needs initial Harness CLI setup. Two things are empty:

1. `.harness-cli/spec/` still contains template placeholders
2. `.harness-cli/kb/` either doesn't exist or has no module documents

Let me walk you through the initialization. We'll do it in this order:
- **First**: Initialize the knowledge base (`kb/`) so AI understands what the product is
- **Second**: Fill in the coding guidelines (`spec/`) so AI knows how to write new code

Ready? Let's start with the knowledge base."

Then run the following step-by-step:

```bash
# Step 1: Create the kb/ directory skeleton
harness-cli scan
```

Expected output:
```
  Creating KB directory structure...
    Created kb/prd/index.md
    Created kb/prd/_module-template.md
    Created kb/tech/index.md
    Created kb/tech/_module-template.md
  KB directory structure created!
```

```bash
# Step 2: Run the AI-side full architecture scan
# (This is a slash command, not a shell command)
/hc-scan-kb-tech
```

This command:
- Reads the template to understand the required format
- Analyzes the whole codebase (tech stack, components, data structures, decisions, cross-cutting)
- Generates 5 docs in `kb/tech/`: `overview.md`, `component-map.md`, `data-models.md`, `decisions.md`, `cross-cutting.md`
- Updates `kb/tech/index.md`

```bash
# Step 3: Seed kb/prd/ with per-module product knowledge
/hc-update-kb
```

On a fresh kb, `/hc-update-kb` catalogs every source file into module documents and creates one markdown per functional module.

After each step, pause and let the human review the output. AI can describe structure accurately, but only the team knows the historical "why."

Once kb/ is populated, move to spec/:

"Now let's fill in the coding guidelines. Look at your existing codebase and extract the patterns already in use."

Work through one file at a time:
- `backend/directory-structure.md`
- `backend/database-guidelines.md`
- `backend/error-handling.md`
- `backend/quality-guidelines.md`
- `backend/logging-guidelines.md`
- `frontend/directory-structure.md`
- `frontend/component-guidelines.md`
- `frontend/hook-guidelines.md`
- `frontend/state-management.md`
- `frontend/quality-guidelines.md`
- `frontend/type-safety.md`

For each file: analyze the codebase, document what you observe (not ideals), include real examples, and list anti-patterns the team avoids.

### Situation B: kb/ Ready but spec/ Empty

Explain to the developer:

"Good news: your knowledge base (`kb/`) is already populated. But your coding guidelines (`spec/`) still contain template placeholders.

This means AI knows what the product does, but doesn't know how the team writes code. Let's fix that by filling in the spec files one at a time."

Proceed with the spec/ filling workflow (same file list as Situation A).

### Situation C: spec/ Filled but kb/ Missing or Stale

Explain to the developer:

"Your team has already customized the coding guidelines. But the product knowledge base (`kb/`) is empty or missing. This means AI can write code in the right style but doesn't know what the existing code does.

Let's initialize the knowledge base:"

```bash
harness-cli scan
```

```bash
/hc-scan-kb-tech
```

```bash
/hc-update-kb
```

### Situation D: Both spec/ and kb/ Are Ready

Explain to the developer:

"Excellent! Both your coding guidelines and knowledge base are already in place. You can start using any slash command right away.

I recommend:
- Read `.harness-cli/spec/` to familiarize yourself with team coding standards
- Read `.harness-cli/kb/tech/overview.md` for a quick architecture tour
- Run `/hc-update-kb` occasionally after merging large changes to keep knowledge fresh"

---

## Completing the Onboard Session

After covering all four parts, summarize:

"You're now onboarded to the Harness CLI workflow system! Here's what we covered:
- Part 1: Core concepts (4 challenges, 3 knowledge pillars: spec / kb / tasks)
- Part 2: Command deep dive (8 core commands + 4 advanced commands)
- Part 3: Real-world examples (bug fix, planning, CR, refactoring, debug, first-time onboarding, parallel feature)
- Part 4: Project foundation check (spec/ and kb/ initialization status)

**Next steps** (tell user):
1. Run `/hc-record-session` to record this onboard session
2. [If foundation incomplete] Finish the spec/ and kb/ initialization we started
3. [If foundation ready] Start your first development task with `/hc-start` + `/hc-brainstorm` or `/hc-before-dev`

What would you like to do first?"
