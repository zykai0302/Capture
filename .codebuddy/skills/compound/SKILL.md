---
description: Capture a structured solution document after implementation to create durable, searchable project knowledge. Use after implementation, verification, and review to turn the just-solved problem into reusable knowledge for future sessions. Creates solution documents under .feature/solutions/<category>/ with proper frontmatter and track structure.
---

# Compound - Capture A Structured Solution While Context Is Fresh

Use this command after implementation, verification, and review to turn the just-solved problem into durable, searchable project knowledge.

Before following this command, read `reference/workflow-context.md`. Reuse already-loaded task artifacts, review results, and changed-file context unless they changed.

---

## Goal

Create one structured solution document that future sessions can find quickly when a similar problem, workflow issue, or best practice question appears again.

The primary output is:

- one solution document under `.feature/solutions/<category>/`

Possible secondary updates:

- `.feature/spec/` when the learning should become a standing rule
- `.feature/tasks/<task>/findings.md` when task-local rationale needs to be preserved
- `.feature/workspace/<developer>/` later via `/record-session`

---

## Support Files

Read these on demand when you need them. Do not bulk-load them unless the current step requires it.

- `reference/schema.yaml` — canonical frontmatter fields and enum values
- `reference/yaml-schema.md` — quick reference for track selection and category mapping
- `reference/resolution-template.md` — section structure for new solution documents

Treat `reference/schema.yaml` as the source of truth if `reference/yaml-schema.md` and your memory ever conflict.

---

## Output Location

Write the final solution document to:

```text
.feature/solutions/<category>/<filename>.md
```

Where:

- `<category>` comes from the `problem_type` to category mapping in `reference/yaml-schema.md`
- `<filename>` should be a concise slug, usually `<problem-slug>-<YYYY-MM-DD>.md`

Examples:

- `.feature/solutions/workflow-issues/manual-command-handoff-2026-04-02.md`
- `.feature/solutions/best-practices/shared-command-context-loading-2026-04-02.md`

---

## Preconditions

- The task is implemented or meaningfully progressed
- Validation already ran, or the current verification status is known
- `prd.md`, `task_plan.md`, `findings.md`, and `progress.md` exist

If task memory is missing, run `/init-plan` first.

---

## Read First

```bash
cat .feature/tasks/<task>/prd.md
cat .feature/tasks/<task>/task_plan.md
cat .feature/tasks/<task>/findings.md
cat .feature/tasks/<task>/progress.md
cat .feature/tasks/<task>/testcase.md
cat .feature/tasks/<task>/testcase_checkdetail.md
```

Also inspect:

- the final diff or changed files
- any review findings that were fixed
- the shared support files above when classifying and assembling the solution doc

---

## What To Capture

Focus on the thing that is most worth reusing later:

- the concrete problem that was solved
- why earlier attempts failed or were incomplete
- the actual solution that worked
- why the solution works
- how to prevent the issue or apply the guidance again

Prefer precise evidence over generic advice:

- file paths
- commands
- exact workflow steps
- specific contracts or invariants
- before/after examples when useful

---

## Track Selection

Use `reference/schema.yaml` and `reference/yaml-schema.md` to choose the track from `problem_type`.

### Bug Track

Use when the learning is mainly about a failure or defect:

- `build_error`
- `test_failure`
- `runtime_error`
- `performance_issue`
- `database_issue`
- `security_issue`
- `ui_bug`
- `integration_issue`
- `logic_error`

Bug-track docs require:

- `symptoms`
- `root_cause`
- `resolution_type`

### Knowledge Track

Use when the learning is mainly about guidance, workflow, or practice:

- `best_practice`
- `documentation_gap`
- `workflow_issue`
- `developer_experience`

Knowledge-track docs do not require the bug-only fields above. Use `applies_when` when it helps future readers know when to consult this solution.

---

## Execution Flow

1. Re-read the task artifacts and current diff.
2. Identify the single best candidate learning to preserve.
3. Classify it:
   - bug track
   - knowledge track
4. Read `reference/schema.yaml` to validate frontmatter fields and enum values.
5. Read `reference/yaml-schema.md` to map `problem_type` to `.feature/solutions/<category>/`.
6. Read `reference/resolution-template.md` and assemble the final markdown using the matching track template.
7. Search `.feature/solutions/` for obviously overlapping docs before creating a new one.
8. If an existing doc covers the same problem and same solution, update that doc instead of creating a duplicate.
9. Otherwise create `.feature/solutions/<category>/<filename>.md`.
10. If the new solution reveals a reusable project rule, update `.feature/spec/` too.
11. Add task-local rationale to `.feature/tasks/<task>/findings.md` when needed.
12. Prepare a concise session summary for `/record-session`.

---

## Overlap Rule

Before creating a new solution doc, check whether `.feature/solutions/` already has one that covers the same:

- problem statement
- root cause or rationale
- solution approach
- affected area or module
- prevention guidance

If overlap is high, update the existing doc rather than creating a near-duplicate.

If overlap is moderate, create the new doc but cross-link the related one.

If overlap is low, create the new doc normally.

---

## Required Solution Shape

The final document should follow `reference/resolution-template.md` and include YAML frontmatter that satisfies `reference/schema.yaml`.

At minimum, the new file should have:

```yaml
---
title: ...
date: YYYY-MM-DD
category: ...
module: ...
problem_type: ...
component: ...
severity: ...
tags: [...]
---
```

Then the body should follow the correct track structure:

- bug track:
  - Problem
  - Symptoms
  - What Didn't Work
  - Solution
  - Why This Works
  - Prevention
  - Related Issues
- knowledge track:
  - Context
  - Guidance
  - Why This Matters
  - When to Apply
  - Examples
  - Related

---

## Discoverability Check

Before finishing, ask:

- would a future agent know that `.feature/solutions/` exists?
- would they know how to search it?
- would they know when it is worth consulting?

If not, make the smallest useful discoverability improvement in the project instructions or spec index that future work will naturally read.

---

## Output Checklist

- [ ] One structured solution doc created or updated under `.feature/solutions/`
- [ ] Frontmatter validated against `reference/schema.yaml`
- [ ] Category chosen using `reference/yaml-schema.md`
- [ ] Body structure follows `reference/resolution-template.md`
- [ ] Duplicate or overlapping docs handled intentionally
- [ ] Reusable rules promoted into `.feature/spec/` when needed
- [ ] Task-local rationale preserved in `.feature/tasks/<task>/` when needed
- [ ] Session summary ready for `/record-session`

---

## Next Step

After the solution document is written and any related rule updates are complete:

```
✓ Compound completed
Next command: /record-session
Run it manually.
```
