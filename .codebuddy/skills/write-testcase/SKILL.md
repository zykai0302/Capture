---
name: write-testcase
description: |
  从任务文档生成标准化功能测试用例的skill，专注于用户视角。
  核心原则：(1) 用户视角优先 - 所有测试用例必须描述用户操作和可观察结果；(2) 无技术术语 - 避免函数名、数据库字段、协议名等；
  (3) 逐步验证 - 每个步骤必须有对应的预期结果。
  在/write-plan后使用，生成测试用例后输出到.feature/tasks/<task>/testcase_analysis.md和testcase.md。
  需要识别产品类型（屏控PK/会议HY/信发XF），使用对应的checklist进行验证。
---

# Write Testcase - Generate Functional Test Cases

Generate standardized functional test cases from task documents, focusing on user perspective.

Before running this skill, read `reference/workflow-context.md`. Reuse any unchanged task, workflow, or spec context already loaded in the current task session.

Also re-read `task_plan.md` first so the current goal and phase are back in attention before starting.

---

## When to Use

**In Complex Task Workflow**:

```text
/write-plan -> /write-testcase -> /check-testcase -> /executing-plans or /subagent-work
```

Use this skill after `/write-plan` to generate test cases before implementation:
- Need to generate test cases for a new feature
- Have task documents (prd.md, findings.md) in task directory
- Test-driven development approach

---

## Sequence

```text
/write-plan -> /write-testcase -> /check-testcase
```

---

## Core Principles

1. **User Perspective First**
   All test cases must describe user operations and observable results, NOT technical implementations.

2. **No Technical Jargon**
   Avoid function names, database fields, protocol names, or internal module names in test steps.

3. **Step-by-Step Verification**
   Each step must have a corresponding expected result.

---

## Execution Steps

### Step 1: Read Task Documents

**→ EXECUTE THIS STEP NOW:**

```bash
# Read task memory files
cat .feature/tasks/<task>/prd.md
cat .feature/tasks/<task>/task_plan.md
cat .feature/tasks/<task>/findings.md
cat .feature/tasks/<task>/implementation-plan.md
```

Extract from task documents:
- Feature requirements and goals
- Functional specifications
- Technical constraints
- Architecture decisions

**✓ Step 1 completed: Task documents read**

---

### Step 2: Read Test Case Templates and Specifications

**→ EXECUTE THIS STEP NOW:**

```bash
# Read test case design specifications
cat reference/testcase_spec.md

# Read test case template
cat reference/testcase_template.md

# Read product specifications
cat reference/product_spec.md
```

**✓ Step 2 completed: Templates and specs loaded**

---

### Step 3: Identify Product Type

**→ EXECUTE THIS STEP NOW:**

Based on task requirements, identify product type:

|| Product Type | Keywords | Checklist File |
||--------------|----------|----------------|
|| Screen Control (PK) | ADU/CDU/DMC/A9/DC1801/Distributed KVM/Keyboard | `reference/testcase_checklist_PK.md` |
|| Conference (HY) | Conference tablet/OPS/Camera | `reference/testcase_checklist_HY.md` |
|| Info Publishing (XF) | Info publishing platform/Terminal software | `reference/testcase_checklist_XF.md` |

**Confirmation Prompt**:
```
基于需求分析，本项目属于**[产品类型]**，将使用 `[checklist文件名]` 进行验证。
是否确认？
```

⚠️ **This step requires user confirmation!**

Wait for user response:
- Confirm → Proceed to Step 4
- Deny → Provide manual selection menu

**✓ Step 3 completed: Product type identified**

---

### Step 4: Generate Requirements Analysis Report

**→ EXECUTE THIS STEP NOW:**

Generate `.feature/tasks/<task>/testcase_analysis.md`:

```markdown
# Requirements Analysis Report: <Feature Name>

## 1. Project Background

### Current Issues
<What problems exist now>

### Project Goals
<What we want to achieve>

## 2. Requirements Overview

### Core Objective
<One sentence describing the main goal>

### Applicable Scenarios
- <Scenario 1>
- <Scenario 2>

### Core Features

|| Feature | Description |
||---------|-------------|
|| <Feature 1> | <Description> |

## 3. Functional Requirements

### <Function 1>
- **Interaction Logic**: <How user interacts>
- **Operation Path**: <Menu/Screen path>
- **Expected Result**: <What user sees>

## 4. Functional Specifications

|| Configuration | Limit | Notes |
||---------------|-------|-------|
|| <Config 1> | <Max/Min> | <Notes> |

## 5. Business Design

Describe core business flows from user perspective.

## 6. Human-Interface Changes

- New menu items
- Enum changes
- Navigation logic changes

## 7. Technical Requirements

- Performance metrics
- Compatibility requirements

## 8. Product Type

**Product Type**: [Screen Control/Conference/Info Publishing]
**Checklist**: [checklist file name]

## 9. Risks and Constraints

- Technical risks
- Business constraints
```

**✓ Step 4 completed: Requirements analysis report generated**

---

### Step 5: Design Test Cases

**→ EXECUTE THIS STEP NOW:**

**Design Principles**:

1. Start from user perspective, describe user operations and observable results
2. Avoid technical terminology (no module names, function names, database fields)
3. Test steps must be clear and executable
4. Expected results must correspond one-to-one with test steps

**Test Type Coverage**:

|| Type | Description |
||------|-------------|
|| Functional Test | Verify basic functionality correctness |
|| Combination Test | Verify multi-condition scenarios |
|| Boundary Test | Verify parameter boundary values |
|| Exception Test | Verify error handling |
|| Compatibility Test | Verify multi-environment compatibility |
|| Performance Test | Verify response time |

**Numbering Rules**:
- Format: x.x.x
- First digit: Major feature number
- Second digit: First-level module number
- Third digit: Test case sequence number

**✓ Step 5 completed: Test cases designed**

---

### Step 6: Generate Test Cases Document

**→ EXECUTE THIS STEP NOW:**

Generate `.feature/tasks/<task>/testcase.md`:

```markdown
# <Feature Name> Test Cases

|| Test ID | Test Type | Level 1 Module | Level 2 Module | Level 3 Module | Test Title | Preconditions | Test Steps | Expected Results | Source | Priority |
||---------|-----------|----------------|----------------|----------------|------------|---------------|------------|------------------|--------|----------|
|| 1.1.1 | Functional Test | <Module> | <Sub-module> | <Optional> | <Title> | <State> | 1. Step 1<br/>2. Step 2 | 1. Result 1<br/>2. Result 2 | AI | H |
```

**Field Filling Rules**:

- **Test ID**: Number by module order
- **Test Type**: Functional/Exception/Boundary/Compatibility/etc.
- **Level 1 Module**: Functional domain classification
- **Level 2 Module**: Specific feature point
- **Level 3 Module**: Optional subdivision
- **Test Title**: Concise description of test goal
- **Preconditions**: User perspective state description
- **Test Steps**: Numbered operation steps (use `<br/>` for line breaks)
- **Expected Results**: Numbered observation results
- **Source**: AI output
- **Priority**: H/M/L (ratio 3:5:2)

**✓ Step 6 completed: Test cases document generated**

---

### Step 7: Update task.json

**→ EXECUTE THIS STEP NOW:**

Edit `.feature/tasks/<task>/task.json` to record test case artifacts:

```json
{
  "status": "testcase-written",
  "artifacts": ["prd.md", "task_plan.md", "findings.md", "progress.md", "implementation-plan.md", "testcase_analysis.md", "testcase.md"],
  "testcaseGenerated": true,
  "productType": "<Screen Control/Conference/Info Publishing>"
}
```

**✓ Step 7 completed: task.json updated**

---

## Output Documents

|| File | Path | Description |
||------|------|-------------|
|| Analysis Report | `.feature/tasks/<task>/testcase_analysis.md` | Structured requirements analysis |
|| Test Cases | `.feature/tasks/<task>/testcase.md` | All test cases in table format |

---

## Quality Checklist

Before completing, verify:

- [ ] All test cases are in a single table
- [ ] Table header order matches template
- [ ] Test steps have clear numbering
- [ ] Expected results correspond one-to-one with steps
- [ ] No technical terminology (database names, function names, etc.)
- [ ] Preconditions described from user perspective
- [ ] Priority distribution is reasonable (H:M:L = 3:5:2)

---

## Key Constraints

- ⚠️ All output files go to `.feature/tasks/<task>/` directory
- ⚠️ Only create 2 output files (testcase_analysis.md, testcase.md)
- ⚠️ Product type must be confirmed with user
- ⚠️ Test cases only get requirements from task documents
- ⚠️ No technical terms in test cases
- ⚠️ Use `replace_in_file` for updates; if it fails, re-read and retry

---

## Workflow Integration

**Simple Task Flow**:
```text
/init-plan -> /write-testcase -> implement -> /check -> /finish-work
```

**Complex Task Flow**:
```text
/brainstorm -> /init-plan -> /research -> /write-plan -> /write-testcase -> /check-testcase -> /executing-plans or /subagent-work
```

---

## Next Step

After test case generation:

```text
✓ Test cases generated
  - Analysis: .feature/tasks/<task>/testcase_analysis.md
  - Test cases: .feature/tasks/<task>/testcase.md
  - Product type: <type>

Next command: /check-testcase
Run it manually to verify completeness with product checklist.
```
