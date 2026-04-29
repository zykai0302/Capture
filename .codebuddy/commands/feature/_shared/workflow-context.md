# Shared Workflow Context

Use this file as the canonical loading and handoff policy for the integrated `/feature:*` command flow.

---

## Shared Loading Policy

When a task stays in the same session and task directory:

- Reuse context that was already loaded by the previous Feature command.
- Do **not** re-read unchanged files just because the next command mentions them again.
- Re-read only when one of these is true:
  - the active task changed
  - the file changed since it was last read
  - the current command needs a file that has not been loaded yet
  - the prior summary is no longer sufficient for a safe decision

Treat these as the main shared context sources:

- session-level context: `.feature/workflow.md`, `python3 ./.feature/scripts/get_context.py`
- task-level context: `.feature/tasks/<task>/task.json`, `prd.md`, `task_plan.md`, `findings.md`, `progress.md`, `implementation-plan.md`, `testcase_analysis.md`, `testcase.md`, `testcase_checkdetail.md`
- spec context: `.feature/spec/frontend/index.md`, `.feature/spec/backend/index.md`, `.feature/spec/guides/index.md`, `.feature/spec/unit-test/index.md`, and the specific guideline files named by the relevant checklist

Prefer referencing the conclusions from the previous command over duplicating the same file reads.

---

## Command Handoff Rule

Every Feature command in the integrated workflow must end the same way:

- name the immediate next recommended `/feature:*` command
- say explicitly that the **user must run it manually**
- do **not** auto-run the next Feature command
- do **not** say "execute the next command immediately" or "automatically proceed"

If more than one next step is possible, present the choices and ask the user to pick by running the relevant command themselves.

---

## Standard Ending

Use this shape at the end of workflow commands:

```text
✓ <current command outcome>
Next command: /feature:<name>
Run it manually when you are ready.
```

For branch points:

```text
✓ <current command outcome>
Next command options:
- /feature:<option-a>
- /feature:<option-b>
Choose one and run it manually.
```
