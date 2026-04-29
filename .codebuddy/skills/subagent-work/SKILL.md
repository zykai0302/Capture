---
description: Execute the plan with fresh subagents per task. Delegates tasks to specialized agents with isolated context, ensuring they stay focused and succeed. Uses fresh subagent per task plus two-stage review (spec then quality) for high quality and fast iteration. Falls back to executing-plans if platform lacks subagent support.
---

# Subagent Work - Execute The Plan With Fresh Subagents Per Task

Use this when the implementation plan is clear and plan items are independent enough to delegate safely.
**Why subagents:** You delegate tasks to specialized agents with isolated context. By precisely crafting their instructions and context, you ensure they stay focused and succeed at their task. They should never inherit your session's context or history — you construct exactly what they need. This also preserves your own context for coordination work.

**Core principle:** Fresh subagent per task + two-stage review (spec then quality) = high quality, fast iteration

Before following this command, read `reference/workflow-context.md`. Reuse already-loaded task and plan context instead of re-reading unchanged files for every delegated step.
Also re-read `task_plan.md` before dispatching work. The controller should keep the roadmap in view even though each subagent gets a narrower packet.

This command integrates the practical parts of subagent-driven development into the Feature task workflow. If the current platform does not support reliable subagents, fall back to `/executing-plans`.

---

## Preconditions

- `prd.md` exists
- `task_plan.md`, `findings.md`, and `progress.md` exist
- `implementation-plan.md` is concrete enough to hand off task by task
- `testcase.md` and `testcase_checkdetail.md` exist (for complex tasks)

If not, run:

1. `/init-plan`
2. `/research`
3. `/before-dev`
4. `/write-plan`
5. `/write-testcase`
6. `/check-testcase`

---

## task.json Status Update

Before starting subagent execution:

Update `.feature/tasks/<task>/task.json` so it records:
- `status: "executing"`
- `executionMode: "subagent"`

Resulting `task.json`:
```json
{
  "status": "executing",
  "executionMode": "subagent"
}
```

---

## Core Principles

- Use a fresh subagent for each implementation task
- Give the subagent only the context it needs for that task
- Keep the controller responsible for coordination and acceptance
- Review every delegated result before moving on
- Update persistent task memory after each task
- Treat `task_plan.md`, `findings.md`, and `progress.md` as the source of continuity between delegated tasks

Do not let subagents improvise scope from vague instructions.

---

## Recommended Delegation Loop

For each plan item:

1. Extract the exact task text from `implementation-plan.md`.
2. Build a task packet for the subagent:
   - task goal
   - exact files or modules involved
   - current phase from `task_plan.md`
   - relevant PRD and findings excerpts
   - relevant prior attempts or status from `progress.md`
   - required validation command
   - any constraints from `.feature/spec/`
3. Dispatch a fresh subagent or isolated execution context.
4. Have it implement only that task.
5. Review the result against:
   - `prd.md`
   - `implementation-plan.md`
   - relevant spec requirements
6. If issues remain, send the same task back for correction or re-dispatch with tighter instructions.
7. Record outcomes in:
   - `task_plan.md`
   - `findings.md`
   - `progress.md`
8. Move to the next task only when the current one is accepted.

When updating the files:

- `task_plan.md` = phase/task status
- `findings.md` = durable new discoveries
- `progress.md` = what the delegate did, what passed, what failed

---

## Review Order

Review delegated work in this order:

1. spec and requirement compliance
2. correctness and code quality
3. validation and test results

Do not start quality polish review while basic requirement compliance is still open.

---

## Model / Agent Selection Guidance

Use lighter agents for:

- isolated implementation steps
- small file edits
- mechanical refactors with clear acceptance criteria

Use stronger agents for:

- multi-file integration tasks
- debugging with unclear root cause
- contract changes or architectural judgment
- review passes that require broad context

---

## When Not To Delegate

Do not use subagent work when:

- tasks are tightly coupled and share the same write surface
- the plan is still vague
- the next step depends on unresolved reasoning by the controller
- the platform cannot support reliable subagent execution

In those cases, use `/executing-plans`.

---

## Required Tracking

After each delegated task, update:

```bash
cat .feature/tasks/<task>/task_plan.md
cat .feature/tasks/<task>/findings.md
cat .feature/tasks/<task>/progress.md
```

At minimum, record:

- task status
- files changed
- tests run
- review findings
- unresolved risks

If a delegated task fails more than once, record the failed approach before re-dispatching. Do not repeatedly retry without updating task memory.

---

## Next Step

After all delegated tasks finish, hand off to the next command:

```
✓ All delegated tasks completed
Next command: /check
Run it manually.
```
