---
description: Formal multi-dimension review that performs stricter review pass than check command. Reviews correctness, maintainability, testing, security, performance, project standards, and blast radius/affected processes. Produces findings list ordered by severity and decides whether task may proceed to finish-work.
---

# Review - Formal Multi-Dimension Review

Perform a stricter review pass than `/check`.

Before following this command, read `reference/workflow-context.md`. Reuse already-loaded PRD, plan, findings, progress, and relevant specs unless any of them changed.

## Review Dimensions

- correctness
- maintainability
- testing
- security
- performance
- project standards
- blast radius and affected processes

---

## Procedure

1. Read the implementation outputs:

```bash
cat .feature/tasks/<task>/prd.md
cat .feature/tasks/<task>/implementation-plan.md
cat .feature/tasks/<task>/findings.md
cat .feature/tasks/<task>/progress.md
cat .feature/tasks/<task>/testcase.md
cat .feature/tasks/<task>/testcase_checkdetail.md
```

2. Review the changed code against:
   - requirements in `prd.md`
   - constraints in `findings.md`
   - test cases in `testcase.md`
   - project rules in `.feature/spec/`

3. Produce a findings list ordered by severity.

4. Decide whether the task may proceed to `/finish-work`.

## Output Format

```markdown
## Review Findings

### High
- ...

### Medium
- ...

### Low
- ...

### Decision
- ready for finish-work | changes required
```

---

## Next Step

After review, hand off based on the result:

```
If review passes:
✓ Review completed
Next command: /finish-work
Run it manually.
```

If review does not pass, hand off back to the execution step that needs more work, usually `/executing-plans` or `/subagent-work`, and tell the user to run it manually.
