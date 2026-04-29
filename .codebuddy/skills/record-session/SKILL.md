---
description: Record work progress after human has tested and committed code. Archives completed tasks to .feature/tasks/archive/YYYY-MM/<task-name>/, adds session to journal via add_session.py script which auto-updates index.md with session stats. Should only be used AFTER the human has tested and committed the code.
---

[!] **Prerequisite**: This command should only be used AFTER the human has tested and committed the code.

Before following this command, read `reference/workflow-context.md`. Reuse the current task summary and git context if they are already loaded and still current.

**Do NOT run `git commit` directly** — the scripts below handle their own commits for `.feature/` metadata. You only need to read git history (`git log`, `git status`, `git diff`) and run the Python scripts.

---

## Record Work Progress

### Step 1: Get Context & Check Tasks

```bash
python3 ./.feature/scripts/get_context.py --mode record
```

Before archiving, verify the task documents already live under `.feature/tasks/<task>/`:

- `prd.md`
- `task_plan.md`
- `findings.md`
- `progress.md`
- `implementation-plan.md` if planning was used
- `testcase_analysis.md`
- `testcase.md`
- `testcase_checkdetail.md`

[!] Archive tasks whose work is **actually done** — judge by work status, not the `status` field in task.json:

- Code committed? → Archive it (don't wait for PR)
- All acceptance criteria met? → Archive it
- Don't skip archiving just because `status` still says `planning` or `in_progress`

```bash
python3 ./.feature/scripts/task.py archive <task-name>
```

This moves the task into:

```text
.feature/tasks/archive/YYYY-MM/<task-name>/
```

### Step 2: One-Click Add Session

```bash
# Method 1: Simple parameters
python3 ./.feature/scripts/add_session.py \
  --title "Session Title" \
  --commit "hash1,hash2" \
  --summary "Brief summary of what was done"

# Method 2: Pass detailed content via stdin
cat << 'EOF' | python3 ./.feature/scripts/add_session.py --stdin --title "Title" --commit "hash"
| Feature | Description |
|---------|-------------|
| New API | Added user authentication endpoint |
| Frontend | Updated login form |

**Updated Files**:
- `packages/api/modules/auth/router.ts`
- `apps/web/modules/auth/components/login-form.tsx`
EOF
```

**Auto-completes**:

- [OK] Appends session to journal-N.md
- [OK] Auto-detects line count, creates new file if >2000 lines
- [OK] Updates index.md (Total Sessions +1, Last Active, line stats, history)
- [OK] Auto-commits .feature/workspace and .feature/tasks changes

---

## Next Step

This is normally the end of the integrated workflow.

If more work remains on another task, hand off to:

```text
✓ Session recorded
Next command: /start-task
Run it manually when you are ready to begin the next task.
```

---

## Script Command Reference

| Command                                                                  | Purpose                                 |
| ------------------------------------------------------------------------ | --------------------------------------- |
| `python3 ./.feature/scripts/get_context.py --mode record`                | Get context for record-session          |
| `python3 ./.feature/scripts/add_session.py --title "..." --commit "..."` | **One-click add session (recommended)** |
| `python3 ./.feature/scripts/task.py archive <name>`                      | Archive completed task (auto-commits)   |
| `python3 ./.feature/scripts/task.py list`                                | List active tasks                       |
