# Check - Development-Phase Quality Check

Check whether the current implementation still follows the relevant Feature specs and test cases before formal review.

Before following this command, read `_shared/workflow-context.md`. Reuse already-loaded package discovery, specs, findings, test cases, and progress unless something changed.

---

## Execution Flow

1. Identify changed files:
   ```bash
   git diff --name-only HEAD
   ```
2. Determine which spec modules apply based on the changed files. Read the relevant spec indexes:
   ```bash
   cat .feature/spec/frontend/index.md
   cat .feature/spec/backend/index.md
   cat .feature/spec/guides/index.md
   ```
3. Read only the specific guideline files referenced by those quality checks and not already loaded.
4. Verify against test cases:
   ```bash
   cat .feature/tasks/<task>/testcase.md
   cat .feature/tasks/<task>/testcase_checkdetail.md
   ```
5. Run the required validation commands for the affected package, typically lint, typecheck, and tests where the spec requires them.
6. Report violations, fix them directly when possible, and record any non-trivial findings in `.feature/tasks/<task>/progress.md` or `findings.md`.

---

## Next Step

After the development-phase check:

```text
✓ Check completed
Next command: /feature:review
Run it manually.
```
