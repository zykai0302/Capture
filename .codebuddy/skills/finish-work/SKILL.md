---
description: Pre-commit checklist to ensure work completeness before submitting or committing. Runs code quality checks (lint, type-check, test), GitNexus refresh, code-spec sync verification, API/DB change verification, cross-layer verification, manual testing checks, and task archive readiness assessment.
---

# Finish Work - Pre-Commit Checklist

Before submitting or committing, use this checklist to ensure work completeness.

**Timing**: After code is written and tested, before commit

Before following this command, read `reference/workflow-context.md`. Reuse already-loaded task artifacts, findings, and spec context instead of re-reading unchanged files.
Also re-read `task_plan.md`, `findings.md`, and `progress.md` explicitly. `finish-work` should verify that the disk-backed task memory and the actual code state still agree.

---

## Checklist

### 1. Code Quality

```bash
# Must pass
pnpm lint
pnpm type-check
pnpm test
```

- [ ] `pnpm lint` passes with 0 errors?
- [ ] `pnpm type-check` passes with no type errors?
- [ ] Tests pass?
- [ ] No `console.log` statements (use logger)?
- [ ] No non-null assertions (the `x!` operator)?
- [ ] No `any` types?

### 1.5. GitNexus Refresh

If the repository uses GitNexus, refresh the analysis output so `.gitnexus/` matches the current code before finishing.

On Windows PowerShell, execution policy may block direct `npx` usage. If that happens, run GitNexus through `cmd /c` instead of invoking `npx` directly from PowerShell.

Preferred order:

1. run local `cmd /c gitnexus analyze` if the CLI is available
2. otherwise use the pinned npx package spec or configured version through `cmd /c`, for example `cmd /c npx -y gitnexus analyze`
3. review `.gitnexus/` changes and keep them if they reflect the current codebase

- [ ] GitNexus analysis refreshed after the latest meaningful code changes?
- [ ] `.gitnexus/` changes reviewed and included when expected?
- [ ] If GitNexus could not run, was that called out explicitly?

### 2. Code-Spec Sync

**Code-Spec Docs**:

- [ ] Does `.feature/spec/backend/` need updates?
  - New patterns, modules, tests, conventions, or contracts
- [ ] Does `.feature/spec/frontend/` need updates?
  - New patterns, modules, tests, conventions, or contracts
- [ ] Does `.feature/spec/guides/` need updates?
  - New cross-layer flows, lessons from bugs

**Key Question**:

> "If I fixed a bug or discovered something non-obvious, should I document it so future me (or others) won't hit the same issue?"

If YES -> Update the relevant code-spec doc.

### 2.2. Task Artifact Completeness

- [ ] `prd.md` is up to date in `.feature/tasks/<task>/`
- [ ] `task_plan.md` reflects the final phase status
- [ ] `findings.md` contains key constraints, decisions, and risks
- [ ] `progress.md` records commands run, test results, and open follow-ups
- [ ] `implementation-plan.md` is updated if the actual implementation diverged from the plan
- [ ] The three task memory files still answer the reboot questions clearly:
  - where am I?
  - what have I learned?
  - what have I done?

### 2.5. Code-Spec Hard Block (Infra/Cross-Layer)

If this change touches infra or cross-layer contracts, this is a blocking checklist:

- [ ] Spec content is executable (real signatures/contracts), not principle-only text
- [ ] Includes file path + command/API name + payload field names
- [ ] Includes validation and error matrix
- [ ] Includes Good/Base/Bad cases
- [ ] Includes required tests and assertion points

**Block Rule**:
If infra/cross-layer changed but the related spec is still abstract, do NOT finish. Run `/update-spec` manually first.

### 3. API Changes

If you modified API endpoints:

- [ ] Input schema updated?
- [ ] Output schema updated?
- [ ] API documentation updated?
- [ ] Client code updated to match?

### 4. Database Changes

If you modified database schema:

- [ ] Migration file created?
- [ ] Schema file updated?
- [ ] Related queries updated?
- [ ] Seed data updated (if applicable)?

### 5. Cross-Layer Verification

If the change spans multiple layers:

- [ ] Data flows correctly through all layers?
- [ ] Error handling works at each boundary?
- [ ] Types are consistent across layers?
- [ ] Loading states handled?

### 6. Manual Testing

- [ ] Feature works in browser/app?
- [ ] Edge cases tested?
- [ ] Error states tested?
- [ ] Works after page refresh?

### 7. Task Archive Readiness

- [ ] The task is actually complete, not just paused
- [ ] All generated task docs are already stored under `.feature/tasks/<task>/`
- [ ] The task is ready to move to `.feature/tasks/archive/<task>/`

---

## Quick Check Flow

```bash
# 0. Re-read task memory
cat .feature/tasks/<task>/task_plan.md
cat .feature/tasks/<task>/findings.md
cat .feature/tasks/<task>/progress.md

# 1. Code checks
pnpm lint && pnpm type-check

# 2. View changes
git status
git diff --name-only

# 2.5. Refresh GitNexus artifacts if used in this repo
cmd /c gitnexus analyze
# or: cmd /c npx -y gitnexus analyze
# 3. Based on changed files, check relevant items above
# 4. After all checks and GitNexus refresh succeed, create the commit
git commit -m "type(scope): description"
```

---

## Common Oversights

| Oversight                      | Consequence                                   | Check                                         |
| ----------------------------- | --------------------------------------------- | --------------------------------------------- |
| Code-spec docs not updated    | Others don't know the change                  | Check .feature/spec/                          |
| Spec text is abstract only    | Easy regressions in infra/cross-layer changes | Require signature/contract/matrix/cases/tests |
| GitNexus index not refreshed  | `.gitnexus/` no longer matches current code   | Run analyze and review `.gitnexus/`           |
| Migration not created         | Schema out of sync                            | Check db/migrations/                          |
| Types not synced              | Runtime errors                                | Check shared types                            |
| Tests not updated             | False confidence                              | Run full test suite                           |
| Console.log left in           | Noisy production logs                         | Search for console.log                        |
| Task memory stale             | Wrong resume point / hidden unfinished work   | Re-read task_plan.md, findings.md, progress.md |

---

## Relationship to Other Commands

```
Development Flow:
  Write code -> Test -> (/impact optional) -> /check -> /review -> /finish-work -> confirm checks succeed -> git commit -> /compound -> /record-session
                                                                                              |                               |
                                                                                       Ensure completeness              Record progress + archive task

Test-Driven Development Flow:
  /write-plan -> /write-testcase -> /check-testcase -> /executing-plans or /subagent-work -> /check -> /review -> /finish-work

Debug Flow:
  Hit bug -> Fix -> /break-loop -> Knowledge capture
                       |
                  Deep analysis
```

- `/finish-work` - Check work completeness (this command)
- `/impact` - Optional blast-radius check before major changes or before final verification
- `/review` - Formal multi-dimension review
- `/compound` - Distill learnings into spec / task / workspace docs
- `/record-session` - Record session and commits
- `/break-loop` - Deep analysis after debugging

---

## Core Principle

> **Delivery includes not just code, but also documentation, verification, knowledge capture, and keeping repo analysis artifacts current.**

Complete work = Code + Docs + Tests + Verification + Refreshed Repo Knowledge

---

## Next Step

After finishing work, hand off to the next command:

```
✓ Work finished (ready to commit)
Next command: /compound
Run it manually.
```
