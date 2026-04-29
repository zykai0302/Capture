# Research - Formal Codebase Investigation Before Planning

Run a dedicated research pass after requirements are clear and before writing the implementation plan.

Before following this command, read `_shared/workflow-context.md`. Reuse unchanged workflow, task, and spec context from the immediately preceding Feature step.
Also re-read `task_plan.md` first so the current goal and phase are back in attention before you start searching.

This command integrates multiple research lenses into one Feature command:

- repository structure and implementation patterns
- existing learnings and prior task memory
- git history where historical context matters
- framework or dependency docs when the task depends on them
- issue intelligence when user-reported pain or recurring bugs affect the scope

You can execute these as explicit sub-research tracks if the platform supports it, or as a single integrated research pass in the current session.

---

## ⚠️ EXECUTION REQUIREMENTS

**You MUST execute ALL steps in order. Do NOT skip steps.**

**Before starting**:
- Verify task_plan.md exists: `cat .feature/tasks/<task>/task_plan.md`

**During execution**:
- Execute each Step in sequence
- Display "✓ Step N completed" after each step
- At the end, recommend the next command without auto-running it

**After completion**:
- findings.md must contain research results
- task.json status must be updated to "researching"

---

## Sequence

```text
/feature:brainstorm -> /feature:init-plan -> /feature:research -> /feature:before-dev -> /feature:write-plan -> /feature:write-testcase -> /feature:check-testcase -> /feature:executing-plans or /feature:subagent-work
```

`brainstorm` may include light pre-research, but this command is where formal reusable findings are written down.

For complex tasks, `write-testcase` and `check-testcase` are mandatory steps between `write-plan` and `executing-plans`/`subagent-work`.

---

## Step 1: Read Input Files

**→ EXECUTE THIS STEP NOW:**

```bash
cat .feature/tasks/<task>/task_plan.md
cat .feature/tasks/<task>/prd.md
```

Re-read `.feature/workflow.md` only if the session context changed or the workflow rules are not already in view.

**✓ Step 1 completed: Input files read**

---

## Step 2: Research Using GitNexus MCP

**→ EXECUTE THIS STEP NOW:**

Use GitNexus MCP tools to research the codebase:

1. **Check repository is indexed**:
   - Tool: `mcp__gitnexus__list_repos`
   - Purpose: Verify repository is available

2. **Query for relevant code**:
   - Tool: `mcp__gitnexus__query`
   - Parameters: concept, keywords related to your task
   - Purpose: Find similar implementations, patterns

3. **Get detailed context**:
   - Tool: `mcp__gitnexus__context`
   - Parameters: key symbols from query results
   - Purpose: Understand call chains, dependencies

4. **Analyze impact** (if modifying existing code):
   - Tool: `mcp__gitnexus__impact`
   - Parameters: symbols you plan to modify
   - Purpose: Understand blast radius

**If GitNexus is not available**, fallback to standard tools:
- `search_file` - Pattern-based file discovery
- `search_content` - Text/regex search
- `read_file` - Direct file inspection

**✓ Step 2 completed: Codebase researched**

---

## Step 3: Update findings.md

**→ EXECUTE THIS STEP NOW:**

Write research results to `findings.md`:

```markdown
## Research Summary

### Relevant Specs
- <path>: <why it matters>

### Existing Patterns
- <pattern>: <example file path>

### Candidate Files To Modify
- <path>: <planned change>

### Constraints
- <technical, workflow, or platform constraints>

### Risks
- <specific ways this task can go wrong>

### Open Questions
- <anything still unclear before planning>
```

Rules:

- external or browser-derived content belongs in `findings.md`, not `task_plan.md`
- if findings change the planned phases, update `task_plan.md` after updating `findings.md`
- after a substantial research pass, note the activity in `progress.md` too

**✓ Step 3 completed: findings.md updated**

---

## Step 4: Update task.json Status

**→ EXECUTE THIS STEP NOW:**

Edit `.feature/tasks/<task>/task.json` so it records that research is in progress or complete.

**Verification**:
```bash
cat .feature/tasks/<task>/task.json
```

Expected output:
```json
{
  "status": "researching",
  "artifacts": ["prd.md", "task_plan.md", "findings.md", "progress.md"],
  "researchCompleted": true
}
```

**✓ Step 4 completed: task.json aligned**

---

## Research Tracks

### 1. Repository And Pattern Research

Identify:

- the package and layer being changed
- existing implementation patterns to copy
- files and modules that define the current workflow
- any platform- or template-specific conventions

Look for concrete examples, not abstract guesses.

### 2. Prior Learnings Search

Search for existing institutional knowledge before inventing a new approach:

- `.feature/spec/`
- `.feature/tasks/` for adjacent task patterns
- `.feature/workspace/` when recent session history is directly relevant

If the repo has a durable learnings store in the future, search that too before proceeding.

### 3. Historical Context

Use git history when any of these are true:

- the code looks strange and you need to know why
- the task touches recently unstable files
- a pattern seems intentional but under-documented
- a regression risk depends on prior behavior

Keep history analysis focused on files and symbols that materially affect the task.

### 4. Framework / Dependency Research

Use this only when the task depends on library behavior, version constraints, or external API details.

Research should answer:

- what version or platform behavior matters
- what official guidance constrains the implementation
- what deprecations, migrations, or best-practice warnings apply

### 5. Issue / Pain Intelligence

Use this when:

- the task is driven by recurring bugs
- user pain patterns should shape the plan
- multiple symptoms may share one root cause

Summarize themes, not just isolated tickets.

---

## Minimum Required Output

Write findings into `.feature/tasks/<task>/findings.md` using a structure like:

```markdown
## Research Summary

### Relevant Specs
- <path>: <why it matters>

### Existing Patterns
- <pattern>: <example file path>

### Candidate Files To Modify
- <path>: <planned change>

### Constraints
- <technical, workflow, or platform constraints>

### Risks
- <specific ways this task can go wrong>

### Open Questions
- <anything still unclear before planning>
```

If historical context, external docs, or issue analysis were used, add dedicated subsections for them.

---

## Research Quality Bar

Good research is:

- specific
- evidenced with file paths or commands
- scoped to the actual task
- useful to planning

Bad research is:

- a broad codebase tour with no task relevance
- restating the PRD
- listing files without saying why they matter
- omitting constraints that will break the plan later

---

## When To Update Other Task Files

If research changes the scope or invalidates prior assumptions:

- update `prd.md`
- update `task_plan.md`
- update `progress.md` with what was researched and why the plan changed

Do not leave the PRD and findings in conflict.

---

## Next Step

After research, hand off to the next command:

```
✓ Research completed (findings.md updated)
Next command: /feature:before-dev
Run it manually.
```
