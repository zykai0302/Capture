# Resolution Templates

## Bug Track Template

```markdown
---
title: [Short descriptive title]
date: YYYY-MM-DD
category: [category-from-schema]
module: [affected-module]
problem_type: [type-from-schema]
component: [specific-component]
severity: [critical|high|medium|low]
tags: [tag1, tag2, tag3]
resolution_type: [code_fix|config_change|dependency_update|workaround|documentation_update]
---

## Problem

[Clear description of the problem that was solved]

## Symptoms

[What symptoms were observed]

- Symptom 1
- Symptom 2

## What Didn't Work

[Approaches that were tried but didn't work]

1. First attempt: [description]
2. Second attempt: [description]

## Solution

[The actual solution that worked]

```[language]
// Code example if applicable
```

## Why This Works

[Explanation of why this solution works]

## Prevention

[How to prevent this issue in the future]

- [ ] Prevention step 1
- [ ] Prevention step 2

## Related Issues

- [Link to related issues or solutions]
```

## Knowledge Track Template

```markdown
---
title: [Short descriptive title]
date: YYYY-MM-DD
category: [category-from-schema]
module: [affected-module]
problem_type: [type-from-schema]
component: [specific-component]
severity: [critical|high|medium|low]
tags: [tag1, tag2, tag3]
applies_when: [When this knowledge applies]
---

## Context

[Background context for this knowledge]

## Guidance

[The actual guidance or best practice]

### Steps

1. Step 1
2. Step 2
3. Step 3

## Why This Matters

[Why this guidance is important]

## When to Apply

[Specific situations where this applies]

- Situation 1
- Situation 2

## Examples

### Example 1: [Title]

```[language]
// Code example
```

## Related

- [Link to related docs or solutions]
```

## Anti-Pattern Warning Template

```markdown
> **Warning**: [Brief description of the anti-pattern]
>
> [More detailed explanation of when this happens and why it's problematic]
```

## Gotcha Template

```markdown
> **Gotcha**: [Brief description of the non-obvious behavior]
>
> [Details about when this happens and how to handle it]
```
