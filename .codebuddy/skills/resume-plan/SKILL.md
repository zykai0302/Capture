---
description: Recover task execution context for an unfinished task-based workflow. Reads task_plan.md, findings.md, progress.md to determine current phase, last completed step, open risks/blockers, and whether the task should continue, split, or be archived. Follows planning-with-files recovery rule.
---

# Resume Plan - Recover Task Execution Context

Use this command to resume an unfinished task-based workflow.

Before following this command, read `reference/workflow-context.md`. Reuse current task context if the same task was already loaded in this session; otherwise reload the task artifacts below.
This command should follow the planning-with-files recovery rule: restore the three task files first, then decide what happens next.

## Recovery Order

1. Read:

```bash
cat .feature/tasks/<task>/task_plan.md
cat .feature/tasks/<task>/findings.md
cat .feature/tasks/<task>/progress.md
```

2. Check current repo state:

```bash
git status --short
python3 ./.feature/scripts/task.py list
```

3. Determine:
   - current phase
   - last completed step
   - open risks or blockers
   - whether the task should continue, split, or be archived
   - whether the task files are still consistent with the current code reality

4. Resume from the next appropriate command:
   - missing research -> `/research`
   - missing plan -> `/write-plan`
   - missing test cases -> `/write-testcase` and `/check-testcase` 
   - implementation incomplete -> `/executing-plans` or `/subagent-work`
   - implementation done -> `/check` or `/finish-work`

## Rule

Do not recreate task documents in a new location. Resume from the existing directory under `.feature/tasks/<task>/`.
Do not skip directly into implementation before restoring `task_plan.md`, `findings.md`, and `progress.md` into working context.

## Next Step

After identifying the current phase, recommend exactly one next command and tell the user to run it manually:

- missing research -> `/research`
- missing plan -> `/write-plan`
- missing test cases -> `/write-testcase` and `/check-testcase` 
- implementation incomplete -> `/executing-plans` or `/subagent-work`
- implementation done but unchecked -> `/check`
- implementation reviewed and ready to wrap -> `/finish-work`
