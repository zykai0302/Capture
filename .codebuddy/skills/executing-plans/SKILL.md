---
description: Execute implementation-plan.md step by step in the current session with persistent task memory. Use when the implementation plan already exists and you want controlled sequential execution instead of delegated subagent execution. Updates task memory on disk after each meaningful action.
---

# Executing Plans - Sequential Plan Execution With Persistent Task Memory

Execute `.feature/tasks/<task>/implementation-plan.md` step by step in the current session.

Before following this command, read `reference/workflow-context.md`. Reuse already-loaded PRD, plan, findings, and progress unless they changed.
Also re-read `task_plan.md` before execution and before major pivots. This is the planning-with-files "read before decide" rule.

Use this when the implementation plan already exists and you want controlled sequential execution instead of delegated subagent execution.

If the platform has strong subagent support and the tasks are mostly independent, prefer `/subagent-work`.

---

## Core Rules

- Review the plan critically before touching code
- Do not skip plan steps silently
- If reality diverges from the plan, update the plan instead of pretending it is still correct
- After each meaningful action, update task memory on disk
- Stop and ask for help when blocked instead of guessing

---

## Required Inputs

Read:

```bash
cat .feature/tasks/<task>/prd.md
cat .feature/tasks/<task>/implementation-plan.md
cat .feature/tasks/<task>/task_plan.md
cat .feature/tasks/<task>/findings.md
cat .feature/tasks/<task>/progress.md
cat .feature/tasks/<task>/testcase.md
cat .feature/tasks/<task>/testcase_checkdetail.md
```

If these files do not exist yet, run:

1. `/init-plan`
2. `/research`
3. `/before-dev`
4. `/write-plan`
5. `/write-testcase`
6. `/check-testcase`

---

## task.json Status Update

Before starting execution:

Update `.feature/tasks/<task>/task.json` so it records:
- `status: "executing"`
- `executionMode: "inline"`

Resulting `task.json`:

```json
{
  "status": "executing",
  "executionMode": "inline"
}
```

---

## Step 1: Load And Review The Plan

Before implementing anything:

1. Read the entire plan.
2. Check each task for:
   - unclear instructions
   - missing files or dependencies
   - validation steps that cannot actually be run
   - plan steps that no longer match the codebase
3. If you find a critical gap, update `implementation-plan.md` and `task_plan.md` first.
4. Only begin execution once the next task is concrete enough to implement safely.

Do not force progress through a bad plan.

---

## Step 2: Execute One Plan Item At A Time

For each unchecked task:

1. Mark it `in_progress` in `task_plan.md` or in the plan itself if that is the tracking source.
2. Follow the plan step exactly.
3. Run the verification or test command specified for that step.
4. Record what happened in `progress.md`:
   - commands run
   - files modified
   - test outcomes
   - blockers or surprises
5. Record durable discoveries in `findings.md`.
6. If completing the item changes the current phase, update `task_plan.md` immediately.
7. Mark the step complete only after validation passes or the exception is explicitly documented.

Do not let `implementation-plan.md`, `task_plan.md`, and `progress.md` drift apart.

---

## When To Update The Plan

Update `implementation-plan.md` immediately when:

- the next steps changed because the codebase differs from the research
- new constraints appeared
- a task had to be split into smaller tasks
- validation commands changed
- the original approach is no longer correct

The plan is a living execution contract, not a historical artifact.

If the roadmap itself changes, update `task_plan.md` too. `implementation-plan.md` is the executable breakdown, but `task_plan.md` remains the top-level orientation file.

---

## Before Large Changes

Before major symbol, contract, schema, or workflow changes, run:

- `/impact`

If the change crosses multiple architectural layers, also consider:

- `/check-cross-layer`

---

## Stop Conditions

Stop executing and ask for clarification when:

- a dependency or environment prerequisite is missing
- a test fails repeatedly and the root cause is unclear
- the plan has a critical ambiguity
- a required verification step cannot be performed
- the requested change now appears broader than the approved plan

Do not guess past blockers.

---

## Next Step

After all plan items are complete, hand off to the next command:

```
✓ Execution completed
Next command: /check
Run it manually.
```

---

## Artifact Location

All persistent planning and execution docs stay under:

```text
.feature/tasks/<task>/
```

Keep `task_plan.md`, `findings.md`, `progress.md`, and `implementation-plan.md` synchronized with reality throughout execution.
