# Check Testcase - Verify Test Case Completeness

Based on product type, verify test case completeness, identify missing test points, and supplement test cases and task documents.

Before running this command, read `_shared/workflow-context.md`. Reuse any unchanged task, workflow, or spec context already loaded in the current task session.

Also re-read `task_plan.md`, `findings.md`, and `progress.md` before verification. The checklist verification should consider the task memory and actual progress.

---

## When to Use

**In Complex Task Workflow**:

```text
/feature:write-plan -> /feature:write-testcase -> /feature:check-testcase -> /feature:executing-plans or /feature:subagent-work
```

Use this command after `/feature:write-testcase`:
- Verify test coverage against product-specific checklist
- Identify and supplement missing test scenarios
- Update task documents with testing context

---

## Preconditions

- ⚠️ `.feature/tasks/<task>/testcase.md` exists
- ⚠️ `.feature/tasks/<task>/testcase_analysis.md` exists
- ⚠️ `.feature/tasks/<task>/task.json` has `testcaseGenerated: true`

---

## Execution Steps

### Step 1: Read Task Documents

**→ EXECUTE THIS STEP NOW:**

```bash
# Read task configuration
cat .feature/tasks/<task>/task.json

# Read task memory files
cat .feature/tasks/<task>/task_plan.md
cat .feature/tasks/<task>/findings.md
cat .feature/tasks/<task>/progress.md
cat .feature/tasks/<task>/prd.md

# Read test case documents
cat .feature/tasks/<task>/testcase_analysis.md
cat .feature/tasks/<task>/testcase.md
```

**✓ Step 1 completed: Task documents read**

---

### Step 2: Confirm Product Type

**→ EXECUTE THIS STEP NOW:**

Read product type from `task.json` or `testcase_analysis.md`.

**Product Type and Checklist Mapping**:

| Product Type | Checklist File |
|--------------|----------------|
| Screen Control (PK) | `_shared/testcase_checklist_PK.md` |
| Conference (HY) | `_shared/testcase_checklist_HY.md` |
| Info Publishing (XF) | `_shared/testcase_checklist_XF.md` |

**Confirmation Prompt**:
```
本项目产品类型为**[产品类型]**，将使用 `[checklist文件名]` 进行验证。
是否确认？
```

⚠️ **This step requires user confirmation!**

Wait for user response:
- Confirm → Proceed to Step 3
- Deny → Provide manual selection menu, then update task.json

**✓ Step 2 completed: Product type confirmed**

---

### Step 3: Read Product Specifications and Checklist

**→ EXECUTE THIS STEP NOW:**

```bash
# Read product specifications
cat _shared/product_spec.md

# Read corresponding checklist
cat _shared/testcase_checklist_<PK|HY|XF>.md
```

**Usage of Product Specs**:

| # | Purpose | Example |
|---|---------|---------|
| 1 | Confirm product model and spec limits | A9 TV wall limit is 16 |
| 2 | Generate boundary test cases | Based on limit 16, create "create 17 TV walls" |
| 3 | Filter relevant modules | A9 supports small-pitch LED → check that module |
| 4 | Generate performance test cases | A9 virtual LED supports 16 → create full-spec test |

**✓ Step 3 completed: Specs and checklist loaded**

---

### Step 4: Filter Relevant Modules

**→ EXECUTE THIS STEP NOW:**

Based on `testcase_analysis.md` and `prd.md`, filter relevant checklist modules:

**Filtering Principles**:

- ⚠️ Involved functional modules MUST be checked
- ⚠️ Related combination modules MUST be checked
- ⚠️ Unrelated modules CAN be skipped (record reason)

**Output**: Generate "modules to check" list

**✓ Step 4 completed: Modules filtered**

---

### Step 5: Execute Module Verification

**→ EXECUTE THIS STEP NOW:**

For each module to check:

1. Read specific checklist section
2. Determine relevance (relevant/irrelevant/partially relevant)
3. Check if test cases cover the checklist items
4. Identify missing test points
5. Update `testcase_checkdetail.md`

**✓ Step 5 completed: Module verification done**

---

### Step 6: Supplement Missing Test Cases

**Method A: Use `replace_in_file` (Preferred)**

Steps:

1. `read_file` last 10-15 lines of `testcase.md`
2. Use last 3-5 lines as `old_str` (ensure uniqueness)
3. `replace_in_file` to append new cases (max 10 per batch)
4. `read_file` to verify success

**Method B: Backup (After 2 consecutive `replace_in_file` failures)**

Steps:

1. `read_file` read **complete file** (no offset/limit)
2. Concatenate original content + new cases
3. Use `write_to_file` to overwrite
4. `read_file` to verify success

**✓ Step 6 completed: Missing test cases supplemented**

---

### Step 7: Generate Checklist Report

**→ EXECUTE THIS STEP NOW:**

Generate `.feature/tasks/<task>/testcase_checkdetail.md`:

```markdown
# Checklist Verification Report

## 1. Product Information

| Item | Content |
|------|---------|
| Product Type | <Screen Control/Conference/Info Publishing> |
| Product Model | <Model> |
| Checklist File | testcase_checklist_<PK|HY|XF>.md |
| Task | <task-slug> |

## 2. Module Coverage Status

| Module Name | Relevance | Coverage Status | Existing Cases | Missing Points |
|-------------|-----------|-----------------|----------------|----------------|
| <Module 1> | Relevant | Fully Covered | 8 | None |
| <Module 2> | Relevant | Partially Covered | 5 | <Missing points> |

## 3. Missing Test Points Summary

### Cases to Supplement

| # | Module | Test Point | Priority | Notes |
|---|--------|------------|----------|-------|
| 1 | <Module> | <Test point> | M | <Notes> |

## 4. Coverage Statistics

| Module | Total Items | Covered | Uncovered | Coverage Rate |
|--------|-------------|---------|-----------|---------------|
| <Module 1> | 4 | 4 | 0 | 100% |
| **Total** | **32** | **27** | **5** | **84%** |

## 5. Conclusion

Current test case coverage is **<%>**.

### Completed Coverage
1. ✅ <Module>: <description>

### Remaining Uncovered
1. <Module>: <description>

**Recommendation**: Supplement <n> test cases.
```

**✓ Step 7 completed: Checklist report generated**

---

### Step 8: Update Task Documents

**→ EXECUTE THIS STEP NOW:**

#### 8.1 Evaluate New Requirements from Test Cases

Assess whether supplemented test cases involve new requirements:

**Evaluation Criteria**:
- Do test cases cover functionality not mentioned in `prd.md`?
- Do test cases require changes to existing requirements?
- Do test cases introduce new acceptance criteria?

**If new requirements identified**:
1. Update `prd.md` - add new functional requirements or modify existing ones
2. Update `task_plan.md` - adjust task scope and deliverables
3. Update `implementation-plan.md` - add implementation steps for new requirements

**If no new requirements**: Proceed to 8.2

**✓ Step 8.1 completed: New requirements evaluated**

#### 8.2 Update prd.md

Add test case reference section if not present:

```markdown
## Test Cases

- Requirements Analysis: `testcase_analysis.md`
- Test Case Document: `testcase.md`
- Verification Report: `testcase_checkdetail.md`
- Product Type: <type>
- Coverage Rate: <%>
```

#### 8.3 Update implementation-plan.md

Add test case execution phase if not present:

```markdown
## Test Case Execution Phase

**Preconditions**:
- Test cases generated and verified
- Coverage rate meets threshold (recommended: >80%)

**Steps**:
- [ ] Run test cases during implementation
- [ ] Record test results
- [ ] Fix failed cases
- [ ] Update test case document if requirements change
```

#### 8.4 Update task_plan.md

Update phase status:

```markdown
## Phase: Test Case Generation

| Phase | Status | Artifacts |
|-------|--------|-----------|
| Write Test Case | ✅ Done | testcase.md, testcase_analysis.md |
| Check Test Case | ✅ Done | testcase_checkdetail.md |
```

#### 8.5 Update findings.md

Add testing findings:

```markdown
## Testing Findings

### Product Type
- Type: <Screen Control/Conference/Info Publishing>
- Checklist: <file>

### Coverage Analysis
- Total test cases: <n>
- Coverage rate: <%>
- Missing points: <list>

### Test Constraints
- <Hardware requirements if any>
- <Environment requirements if any>
```

#### 8.6 Update progress.md

Add progress record:

```markdown
## [Date] Test Case Verification Completed

### Actions
1. Verified test cases against <checklist>
2. Identified <n> missing test points
3. Supplemented <n> test cases
4. Updated task documents

### Artifacts
- testcase_checkdetail.md (new)
- testcase.md (updated)
- prd.md (updated)
- implementation-plan.md (updated)
- task_plan.md (updated)
- findings.md (updated)

### Next
- Proceed to implementation
- Run test cases during development
```

**✓ Step 8 completed: Task documents updated**

---

### Step 9: Update task.json

**→ EXECUTE THIS STEP NOW:**

Edit `.feature/tasks/<task>/task.json`:

```json
{
  "status": "testcase-verified",
  "artifacts": [
    "prd.md",
    "task_plan.md",
    "findings.md",
    "progress.md",
    "implementation-plan.md",
    "testcase_analysis.md",
    "testcase.md",
    "testcase_checkdetail.md"
  ],
  "testcaseGenerated": true,
  "testcaseVerified": true,
  "productType": "<Screen Control/Conference/Info Publishing>",
  "testCoverageRate": "<%>"
}
```

**✓ Step 9 completed: task.json updated**

---

## Output Documents

| File | Path | Description |
|------|------|-------------|
| Checklist Report | `.feature/tasks/<task>/testcase_checkdetail.md` | Detailed coverage analysis |
| Test Cases (Updated) | `.feature/tasks/<task>/testcase.md` | Supplemented missing cases |
| PRD (Updated) | `.feature/tasks/<task>/prd.md` | Added test case section |
| Implementation Plan (Updated) | `.feature/tasks/<task>/implementation-plan.md` | Added test execution phase |
| Task Plan (Updated) | `.feature/tasks/<task>/task_plan.md` | Updated phase status |
| Findings (Updated) | `.feature/tasks/<task>/findings.md` | Added testing findings |
| Progress (Updated) | `.feature/tasks/<task>/progress.md` | Added progress record |

---

## Key Constraints

- ⚠️ All output files go to `.feature/tasks/<task>/` directory
- ⚠️ Prefer Method A (`replace_in_file`), use Method B after 2 failures
- ⚠️ Method B must read complete file before writing
- ⚠️ Max 10 cases per batch append
- ⚠️ Product type must be confirmed with user
- ⚠️ Table format must use single pipe `|`, no double pipe `||`
- ⚠️ Unsupported features marked as "not applicable", not skipped
- ⚠️ All task documents must be updated consistently

---

## Workflow Integration

**Complex Task Flow**:
```text
/feature:brainstorm -> /feature:init-plan -> /feature:research -> /feature:write-plan -> /feature:write-testcase -> /feature:check-testcase -> /feature:executing-plans or /feature:subagent-work
```

Test case verification happens **before** implementation:
- Provides clear testing goals for implementation
- Ensures coverage during development
- Supports test-driven development approach

---

## Next Step

After checklist verification:

```text
✓ Test case verification completed
  - Report: .feature/tasks/<task>/testcase_checkdetail.md
  - Test cases updated: .feature/tasks/<task>/testcase.md
  - Coverage rate: <%>
  - Task documents updated: prd.md, implementation-plan.md, task_plan.md, findings.md, progress.md

Next command options:
- /feature:executing-plans (sequential implementation)
- /feature:subagent-work (delegated implementation)
Choose one and run it manually.
```
