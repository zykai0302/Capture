---
name: write-plan
description: |
  生成可执行实现计划的skill，在需求和研究完成后生成具体的实现计划。
  计划必须可由缺乏项目上下文的工程师执行，包含精确文件、验证步骤、无占位符。
  输出到.feature/tasks/<task>/implementation-plan.md，同时更新task_plan.md。
  计划标准：假设未来实现者没有进行研究、不了解代码库、如果留有空白会做出错误假设。
  必须包含：计划头、每个任务的具体内容、粒度规则（小到可独立验证）、无占位符规则。
  生成后需人工验证门控，等待用户确认后更新task.json状态为"plan-written"。
---

# Write Plan - Produce An Executable Implementation Plan

Generate a concrete implementation plan after requirements and research are complete.

Before running this skill, read `reference/workflow-context.md`. Reuse unchanged PRD, findings, and task memory that are already loaded in the current task session.
Also re-read `task_plan.md` before planning. In the planning-with-files model, this is the top-level roadmap that keeps the goal and current phase in attention.

This skill integrates the strongest parts of `writing-plans`: the plan must be executable by an engineer with little project context, with exact files, validation steps, and no placeholders.

---

## Sequence

Recommended order:

```text
/brainstorm -> /init-plan -> /research -> /before-dev -> /write-plan
```

After writing the plan, for complex tasks requiring test-driven development:

```text
/write-plan -> /write-testcase -> /check-testcase -> /executing-plans or /subagent-work
```

For simple tasks without test case generation:

- `/executing-plans`
- or `/subagent-work`

---

## Inputs

Read:

```bash
cat .feature/tasks/<task>/prd.md
cat .feature/tasks/<task>/findings.md
cat .feature/tasks/<task>/task_plan.md
```

Also inspect the relevant code and specs referenced by research before writing the plan.

Use the three task memory files intentionally:

- `task_plan.md` = roadmap and current phase
- `findings.md` = research, discoveries, constraints
- `progress.md` = what has already been tried or completed

---

## Output Location

Write the plan to:

```text
.feature/tasks/<task>/implementation-plan.md
```

Update `task_plan.md` so the plan phases and execution status are visible there too.

Do not treat `implementation-plan.md` as a replacement for `task_plan.md`.

- `task_plan.md` remains the orientation file
- `implementation-plan.md` is the executable breakdown

---

## Planning Standard

Assume the future implementer:

- did not perform the research themselves
- does not know this codebase well
- will make bad assumptions if you leave gaps

Write the plan so they can execute it safely without inventing missing details.

The plan must also be consistent with the existing task memory:

- if `findings.md` contains a constraint, the plan must honor it
- if `progress.md` shows something already tried or already done, do not plan it as if it were new work
- if the roadmap in `task_plan.md` is no longer right, update `task_plan.md` while writing the plan

---

## Required Plan Header

Start the file with:

```markdown
# <Feature Name> Implementation Plan

**Goal:** <one-sentence outcome>

**Context:**
- task directory: `.feature/tasks/<task>/`
- related PRD: `.feature/tasks/<task>/prd.md`
- key findings: summarize the constraints that matter for execution

**Architecture:** [2-3 sentences about approach]

**Tech Stack:** [Key technologies/libraries]

**Execution Options:**
- with test cases: `/write-testcase` -> `/check-testcase` -> `/executing-plans` or `/subagent-work`
- sequential: `/executing-plans`
- delegated: `/subagent-work`
```

---

## Required Content Per Task

Each task item should include:

- exact target files or modules
- the concrete change to make
- validation or test command
- notable risks, dependencies, or prerequisites
- enough detail to execute without guessing

Good task items are small enough to execute and verify independently.

---

## Granularity Rule

Prefer bite-sized tasks.

Break tasks down so each unit is a meaningful action such as:

- add or update a test
- implement one code path
- wire one contract or configuration
- run one validation step
- perform one focused review or follow-up correction

If a task is too large to validate in one pass, split it.

---

## No Placeholder Rule

Do not write:

- `TODO`
- `TBD`
- `handle edge cases`
- `add validation`
- `write tests`
- `update docs`

unless the exact files, commands, and expected behavior are specified nearby.

If the implementer would need to invent the missing detail, the plan is incomplete.

---

## Recommended Structure

```markdown
## Task 1: <name>

**Why**
- <why this task exists>

**Files**
- Modify: `path/to/file`
- Create: `path/to/new-file`
- Test: `path/to/test-file`

**Steps**
- [ ] <specific implementation step>
- [ ] <specific validation step>

- [ ] **Step 1: Write the failing test**

```python
def test_specific_behavior():
    result = function(input)
    assert result == expected
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pytest tests/path/test.py::test_name -v`
Expected: FAIL with "function not defined"

- [ ] **Step 3: Write minimal implementation**

```python
def function(input):
    return expected
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pytest tests/path/test.py::test_name -v`
Expected: PASS

**Validation**

- Run: `<exact command>`
- Expect: `<expected outcome>`

**Risks / Notes**

- <important dependency, contract, or gotcha>

```

---

## Self-Review Before Saving

Check the plan against the PRD and findings:

1. Every requirement has a matching task
2. File paths are concrete
3. Validation commands are real and runnable
4. Names, types, and contracts are consistent across tasks
5. No placeholder language remains
6. The phase roadmap in `task_plan.md` still matches the plan you are handing off
7. Any prior attempts in `progress.md` are reflected appropriately

Fix the plan inline before handing it off.

After saving the plan:

- update `task_plan.md` with the planned phases or revised phase statuses
- add a short note to `progress.md` that planning completed
- add any newly clarified constraints to `findings.md` if they were discovered during planning

---

## Human Validation Gate (Required)

After generating `implementation-plan.md`, **WAIT for user confirmation**:

**Success path:**

- User confirms plan is accurate and complete

- Update `task.json` status to `"plan-written"`

- Add `"implementation-plan.md"` to `artifacts` array

- **Display the next command options only:**
  
  ```
  ✓ Implementation plan validated
  Next command options:
  - /write-testcase
  - /executing-plans 
  - /subagent-work 
  Choose one and run it manually.
  ```

- Ask the user which execution mode they want, but do not run it automatically
- Recommend `/write-testcase` for complex tasks requiring test-driven development

**Failure path:**

- User indicates plan is inaccurate or needs refinement
- Update `task.json` status back to `"brainstorming"`
- Return to `/brainstorm` to re-clarify requirements
- May need additional `/research` if technical details are unclear

### Update task.json

After successful plan validation:

Update `.feature/tasks/<task>/task.json` so it records the validated implementation plan.

Resulting `task.json`:

```json
{
  "status": "plan-written",
  "artifacts": ["prd.md", "task_plan.md", "findings.md", "progress.md", "implementation-plan.md"],
  "planValidated": true
}
```

---

## Handoff

After saving:

- update `task_plan.md` with the planned phases
- for complex tasks requiring TDD: hand off to `/write-testcase`
- for simple tasks: hand off to `/executing-plans` or `/subagent-work`
- the user must run the chosen command manually

## Next Step

After plan validation:

```text
✓ Implementation plan validated
Next command options:
- /write-testcase 
- /executing-plans 
- /subagent-work 
Choose one and run it manually.
```
