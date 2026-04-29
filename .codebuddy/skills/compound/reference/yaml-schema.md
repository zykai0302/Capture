# YAML Schema Quick Reference

## Problem Type to Category Mapping

| problem_type | category |
|--------------|----------|
| build_error | build-errors |
| test_failure | test-failures |
| runtime_error | runtime-errors |
| performance_issue | performance-issues |
| database_issue | database-issues |
| security_issue | security-issues |
| ui_bug | ui-bugs |
| integration_issue | integration-issues |
| logic_error | logic-errors |
| best_practice | best-practices |
| documentation_gap | documentation |
| workflow_issue | workflow-issues |
| developer_experience | developer-experience |

## Track Selection

### Bug Track
Use when the learning is mainly about a failure or defect.

Required fields:
- `symptoms`: What went wrong
- `root_cause`: Why it went wrong
- `resolution_type`: How it was fixed

### Knowledge Track
Use when the learning is mainly about guidance, workflow, or practice.

Optional fields:
- `applies_when`: When to apply this knowledge

## Required Frontmatter

```yaml
---
title: Short descriptive title
date: YYYY-MM-DD
category: one-of-categories-above
module: affected-module
problem_type: one-of-problem-types-above
component: specific-component
severity: critical | high | medium | low
tags: [tag1, tag2, ...]
---
```

## Optional Fields

```yaml
applies_when: When this knowledge applies
related: [related-solution-ids]
resolution_type: code_fix | config_change | dependency_update | workaround | documentation_update
symptoms: What symptoms were observed
root_cause: What was the root cause
```

## Track-Specific Body Structure

### Bug Track
- Problem
- Symptoms
- What Didn't Work
- Solution
- Why This Works
- Prevention
- Related Issues

### Knowledge Track
- Context
- Guidance
- Why This Matters
- When to Apply
- Examples
- Related
