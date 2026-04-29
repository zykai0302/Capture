# Start Session

Initialize your AI development session and begin working on tasks.

Before following this command, read `_shared/workflow-context.md` and reuse already-loaded session context instead of re-reading unchanged workflow or spec files.

---

## Operation Types

Operations in this document are categorized as:

| Marker | Meaning | Executor |
|--------|---------|----------|
| `[AI]` | Bash scripts or file reads executed by AI | You (AI) |
| `[USER]` | Slash commands executed by user | User |

---

## Initialization

### Step 1: Understand Development Workflow `[AI]`

First, read the workflow guide to understand the development process:

```bash
cat .feature/workflow.md  # Development process, conventions, and quick start guide
```

**Follow the instructions in workflow.md** - it contains:
- Core principles (Read Before Write, Follow Standards, etc.)
- File system structure
- Development process
- Best practices

### Step 2: Get Current Status `[AI]`

```bash
python3 ./.feature/scripts/get_context.py
```

This returns:
- Developer identity
- Git status (branch, uncommitted changes)
- Recent commits
- Active tasks
- Journal file status

### Step 3: Read Guidelines Index `[AI]`

```bash
cat .feature/spec/frontend/index.md  # Frontend guidelines
cat .feature/spec/backend/index.md   # Backend guidelines
cat .feature/spec/guides/index.md    # Thinking guides
```

> **Important**: The index files are navigation — they list the actual guideline files (e.g., `error-handling.md`, `conventions.md`, `mock-strategies.md`).
> At this step, just read the indexes to understand what's available.
> When you start actual development, you MUST go back and read the specific guideline files relevant to your task, as listed in the index's Pre-Development Checklist.

### Step 4: Check Active Tasks `[AI]`

```bash
python3 ./.feature/scripts/task.py list
```

If continuing previous work, review the task file.

### Step 5: Report Ready Status and Ask for Tasks

Output a summary:

```markdown
## Session Initialized

| Item | Status |
|------|--------|
| Developer | {name} |
| Branch | {branch} |
| Uncommitted | {count} file(s) |
| Journal | {file} ({lines}/2000 lines) |
| Active Tasks | {count} |

Ready for your task. What would you like to work on?
```

---

## Task Classification

When user describes a task, classify it:

| Type | Criteria | Workflow |
|------|----------|----------|
| **Question** | User asks about code, architecture, or how something works | Answer directly |
| **Trivial Fix** | Typo fix, comment update, single-line change, < 5 minutes | Direct Edit |
| **Simple Task** | Clear goal, 1-2 files, well-defined scope | Quick confirm -> Task Workflow |
| **Complex Task** | Vague goal, multiple files, architectural decisions | **Brainstorm -> Task Workflow** |

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
> Task Workflow ensures code-specs are injected to the right context, resulting in higher quality code.
> The overhead is minimal, but the benefit is significant.

> **Subtask Decomposition**: If brainstorm reveals multiple independent work items,
> consider creating subtasks using `--parent` flag or `add-subtask` command.
> See `/feature:brainstorm` Step 8 for details.

---

## Question / Trivial Fix

For questions or trivial fixes, work directly:

1. Answer question or make the fix
2. If code was changed, remind user to run `/feature:finish-work`

---

## Simple Task

For simple, well-defined tasks:

1. Quick confirm: "I understand you want to [goal]. Shall I proceed?"
2. If no, clarify and confirm again
3. **If yes: classify the task and hand off to the next command. Do not auto-run later `/feature:*` commands from inside `/feature:start`.**

   ```text
   OK Task classified as: Simple Task
   Next command: /feature:init-plan
   Run it manually.
   ```

For simple development tasks, `/feature:init-plan` is the lightweight workflow entry. It should ensure the task directory exists, seed a minimal `prd.md` if needed, and create or restore the durable task files (`task_plan.md`, `findings.md`, `progress.md`).

Default lightweight flow for simple tasks:

```text
/feature:start -> /feature:init-plan -> implement directly -> /feature:check -> /feature:finish-work -> /feature:record-session
```

Escalate only if needed:

- use `/feature:research` if the codebase or constraints are still unclear
- use `/feature:before-dev` if you need injected spec/context files before editing
- use `/feature:write-plan` if the task stops being simple and now needs an executable plan

---

## Complex Task - Brainstorm First

For complex or vague tasks, route to brainstorm first. Do NOT skip directly to implementation.

Display the handoff and stop:

```text
OK Task classified as: Complex Task
Next command: /feature:brainstorm
Run it manually.
```

The brainstorm command will:
1. Use GitNexus MCP tools to research the codebase in Step 1
2. Guide you through requirements clarification
3. Generate PRD with human validation gate
4. Hand off to `/feature:init-plan` after validation

Summary of what brainstorm does:

1. **Acknowledge and classify** - State your understanding
2. **GitNexus research (Step 1)** - Use `mcp__gitnexus__*` tools to understand codebase
3. **Create task directory** - Track evolving requirements in `prd.md`
4. **Ask questions one at a time** - Update PRD after each answer
5. **Propose approaches** - For architectural decisions
6. **Confirm final requirements** - Get explicit approval (Human Validation Gate)
7. **Hand off to `/feature:init-plan`** - After PRD validation

## Next Step

End `/feature:start` by classifying the task and naming exactly one next user-run command:

- question or trivial fix -> no workflow handoff required
- simple task -> `/feature:init-plan`
- complex task -> `/feature:brainstorm`

---

## Task Workflow (Development Tasks)

**Why this workflow?**
- Keep task memory durable on disk
- Allow simple work to stay lightweight
- Escalate to research and planning only when complexity justifies it
- Result: less ceremony for small changes, without losing recoverability

### Overview: Two Entry Points

```text
From Brainstorm (Complex Task):
  PRD confirmed -> /feature:init-plan -> /feature:research -> /feature:before-dev -> /feature:write-plan -> implement -> check -> complete

From Simple Task:
  Confirm -> /feature:init-plan -> implement directly -> /feature:check -> /feature:finish-work -> /feature:record-session
```

In the simple-task path, "implement directly" means continue coding in the same session after `/feature:init-plan` finishes. It is not another required `/feature:*` command.

**Key principle: simple tasks start light and escalate only when the task stops being simple.**

`before-dev` is not the requirements or task-memory entry point. It assumes the task already has `prd.md`, `task_plan.md`, `findings.md`, and `progress.md`.

### Integrated Workflow Addendum

For substantial tasks, use the fuller sequence:

```text
/feature:brainstorm -> /feature:init-plan -> /feature:research -> /feature:before-dev -> /feature:write-plan -> /feature:write-testcase -> /feature:check-testcase -> /feature:executing-plans or /feature:subagent-work -> optional /feature:impact -> /feature:check -> /feature:review -> /feature:finish-work -> /feature:compound -> /feature:record-session
```

Additional rules:
- `brainstorm` comes before planning and is used to clarify requirements and compare approaches
- `research` is the formal codebase investigation step and belongs before `write-plan`
- simple tasks do not need `research`, `before-dev`, `write-plan`, `write-testcase`, or `check-testcase` by default
- for complex tasks, `write-testcase` and `check-testcase` are mandatory steps between `write-plan` and `executing-plans`
- `impact` is optional and is most useful before major changes or before final verification on risky tasks
- all generated task documents must live under `.feature/tasks/<task>/`

---

### Lightweight Execution Model

For simple tasks, keep the workflow narrow:

1. Confirm the goal and any non-obvious constraint.
2. Run `/feature:init-plan` to ensure the task directory, minimal `prd.md`, and task memory files exist.
3. Implement directly in the current session.
4. Run `/feature:check`.
5. Run `/feature:finish-work`.
6. Run `/feature:record-session`.

Upgrade the task to the substantial workflow if any of these become true:

- the change expands beyond 1-2 files
- you need repository research to avoid guesswork
- the task touches infra or cross-layer contracts
- you need a delegated or multi-phase execution plan

---

## User Available Commands `[USER]`

The following slash commands are for users (not AI):

| Command | Description |
|---------|-------------|
| `/feature:start` | Start development session (this command) |
| `/feature:brainstorm` | Clarify vague requirements before implementation |
| `/feature:init-plan` | Create or restore the task directory, `prd.md`, and task memory files |
| `/feature:research` | Perform formal codebase research before planning |
| `/feature:before-dev` | Read development guidelines |
| `/feature:write-plan` | Generate `.feature/tasks/<task>/implementation-plan.md` |
| `/feature:executing-plans` | Execute the implementation plan sequentially |
| `/feature:subagent-work` | Execute plan items in delegated fashion |
| `/feature:impact` | Review blast radius before major changes |
| `/feature:check` | Check code quality |
| `/feature:review` | Formal multi-dimension review |
| `/feature:check-cross-layer` | Cross-layer verification |
| `/feature:finish-work` | Pre-commit checklist |
| `/feature:compound` | Distill learnings into task / spec / workspace docs |
| `/feature:record-session` | Record session progress |

---

## AI Executed Scripts `[AI]`

| Script | Purpose |
|--------|---------|
| `python3 ./.feature/scripts/task.py create "<title>" [--slug <name>]` | Create task directory |
| `python3 ./.feature/scripts/task.py list` | List active tasks |
| `python3 ./.feature/scripts/task.py archive <name>` | Archive task |
| `python3 ./.feature/scripts/get_context.py` | Get session context |

---

## Platform Detection

Feature auto-detects your platform based on config directories. For CodeBuddy users, ensure detection works correctly:

| Condition | Detected Platform |
|-----------|-------------------|
| Only `.codebuddy/` exists | `codebuddy` |
| Both `.codebuddy/` and `.claude/` exist | `claude` (default) |

If auto-detection fails, set manually:

```bash
export FEATURE_PLATFORM=codebuddy
```

Or prefix commands:

```bash
FEATURE_PLATFORM=codebuddy python3 ./.feature/scripts/task.py list
```

---

## Session End Reminder

**IMPORTANT**: When a task or session is completed, remind the user:

> Before ending this session, please run `/feature:record-session` to record what we accomplished.
