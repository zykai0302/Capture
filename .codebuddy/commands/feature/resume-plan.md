# Resume Plan - Recover Task Execution Context

Use this command to resume an unfinished task-based workflow.

Before following this command, read `_shared/workflow-context.md`. Reuse current task context if the same task was already loaded in this session; otherwise reload the task artifacts below.
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
   - missing research -> `/feature:research`
   - missing plan -> `/feature:write-plan`
   - missing test cases -> `/feature:write-testcase` and `/feature:check-testcase` 
   - implementation incomplete -> `/feature:executing-plans` or `/feature:subagent-work`
   - implementation done -> `/feature:check` or `/feature:finish-work`

## Rule

Do not recreate task documents in a new location. Resume from the existing directory under `.feature/tasks/<task>/`.
Do not skip directly into implementation before restoring `task_plan.md`, `findings.md`, and `progress.md` into working context.

## Next Step

After identifying the current phase, recommend exactly one next command and tell the user to run it manually:

- missing research -> `/feature:research`
- missing plan -> `/feature:write-plan`
- missing test cases -> `/feature:write-testcase` and `/feature:check-testcase` 
- implementation incomplete -> `/feature:executing-plans` or `/feature:subagent-work`
- implementation done but unchecked -> `/feature:check`
- implementation reviewed and ready to wrap -> `/feature:finish-work`
