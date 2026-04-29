Read the relevant development guidelines before starting your task.

Before following the steps below, read `_shared/workflow-context.md`. Reuse unchanged package discovery, task docs, and spec context that was already loaded in the current task session.

---

## ⚠️ EXECUTION REQUIREMENTS

**You MUST execute ALL steps in order. Do NOT skip steps.**

**Before starting**:

- Verify required files exist: `prd.md`, `task_plan.md`, `findings.md`, `progress.md`

**During execution**:

- Execute each Step in sequence
- Display "✓ Step N completed" after each step
- At the end, recommend the next command without auto-running it

**After completion**:

- Context files must be initialized (implement.jsonl, check.jsonl, debug.jsonl)
- Task must be activated (`.current-task` set)

---

## Prerequisites

If these files do not exist yet:

- Missing `prd.md`, `task_plan.md`, `progress.md` → stop and tell the user to run `/feature:init-plan` manually first
- Missing research in `findings.md` → stop and tell the user to run `/feature:research` manually first

`/feature:before-dev` is not the workflow entry for new development tasks. Use it only after the task files and research baseline already exist.

---

## Step 1: Read Spec Indexes

**→ EXECUTE THIS STEP NOW:**

Identify which specs apply to your task and read the index for each:

```bash
cat .feature/spec/frontend/index.md
cat .feature/spec/backend/index.md
cat .feature/spec/guides/index.md
```

Follow the **"Pre-Development Checklist"** section in the index.

**✓ Step 1 completed: Spec indexes read**

---

## Step 2: Read Specific Guideline Files

**→ EXECUTE THIS STEP NOW:**

Read the actual guideline files listed in the Pre-Development Checklist (not just the index):

```bash
cat .feature/spec/guides/<guideline-file>.md
# Example: error-handling.md, conventions.md, etc.
```

Only re-read spec indexes and guideline files that are newly relevant or have changed since the previous workflow step.

**✓ Step 2 completed: Guidelines loaded**

---

## Step 3: Read Task Documents

**→ EXECUTE THIS STEP NOW:**

```bash
cat .feature/tasks/<task>/prd.md
cat .feature/tasks/<task>/task_plan.md
cat .feature/tasks/<task>/findings.md
```

**✓ Step 3 completed: Task documents aligned**

---

## Step 4: Initialize Implementation Context

**→ EXECUTE THIS STEP NOW:**

```bash
python3 ./.feature/scripts/task.py init-context "$TASK_DIR" <type>
# type: backend | frontend | fullstack | test | docs
```

Expected output:

```
Creating implement.jsonl... ✓
Creating check.jsonl... ✓
Creating debug.jsonl... ✓
```

**✓ Step 4 completed: Context files initialized**

---

## Step 5: Add Relevant Specs to Context

**→ EXECUTE THIS STEP NOW:**

Based on your research and guidelines, add relevant specs:

```bash
# For implement context:
python3 ./.feature/scripts/task.py add-context "$TASK_DIR" implement "<spec-path>" "<reason>"

# For check context:
python3 ./.feature/scripts/task.py add-context "$TASK_DIR" check "<spec-path>" "<reason>"
```

**✓ Step 5 completed: Specs added to context**

---

## Next Step

After all steps are completed, hand off to the next command:

```
✓ All steps completed
Next command: /feature:write-plan
Run it manually.
```
