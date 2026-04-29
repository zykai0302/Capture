---
description: Blast radius analysis for major code changes. Analyzes direct callers/callees, related modules/workflows, cross-layer contracts, and validation/deployment risks. Uses GitNexus MCP as preferred method with conventional repo analysis as fallback. Outputs risk level (low/medium/high) and required verification steps.
---

# Impact - Blast Radius Analysis

Run this command before major code changes and again before final completion if needed.

Before following this command, read `reference/workflow-context.md`. Reuse prior research and implementation context if the same task is still active.

## Purpose

Understand what a change may affect:

- direct callers and callees
- related modules and workflows
- cross-layer contracts
- validation, tests, and deployment risk

Prefer GitNexus MCP for impact analysis because it can resolve symbol relationships and process flows more accurately than plain file search. If MCP is unavailable, the repository is not indexed, or the result is insufficient, fall back to conventional repo analysis.

---

## Preferred Method: GitNexus MCP First

When available, follow this sequence:

1. Identify the target symbol, module, command, API, or workflow being changed.
2. Use `mcp__gitnexus__impact` on the target with upstream direction first.
3. Review direct `d=1` dependents as the highest-risk items.
4. Check affected execution flows or processes via GitNexus context/process data when available.
5. Use `mcp__gitnexus__detect_changes` if the task already has local edits and you want a diff-based blast radius check.
6. If GitNexus reports the index is stale, refresh the index before trusting the result.

Suggested MCP-first checklist:

- [ ] `mcp__gitnexus__impact` used for the primary blast-radius query
- [ ] direct `d=1` dependents reviewed first
- [ ] high-confidence dependencies reviewed
- [ ] relevant execution flows/processes checked
- [ ] `mcp__gitnexus__detect_changes` used when local edits already exist

If GitNexus gives only a partial answer, keep the useful parts and continue with the fallback method instead of discarding the MCP result.

---

## Fallback Method: Conventional Repo Analysis

If MCP is unavailable or insufficient:

1. Identify the target symbol, module, command, API, or workflow being changed.
2. Find upstream and downstream dependencies.
3. List files that are likely to break if the change is wrong.
4. Identify missing tests or checks needed to make the change safe.
5. Use normal repo tools as needed:
   - `rg` for symbol and file references
   - `git diff --name-only` for changed-file scope
   - targeted file reads for contracts and call sites
6. Write the result into `.feature/tasks/<task>/findings.md`.

---

## Risk Heuristics

- low
  - only a few direct callers
  - limited surface area
  - no critical workflow involved
- medium
  - multiple dependents or multiple flows touched
  - at least one shared module or command interface
- high
  - broad dependency graph
  - cross-layer contract changes
  - auth/payment/build/release/runtime-critical flow involved

## Output Format

```markdown
## Impact Analysis

### Change Target
- ...

### Affected Files / Modules
- ...

### Affected Flows / Processes
- ...

### Risk Level
- low | medium | high

### Required Verification
- ...

### Method
- GitNexus MCP | conventional fallback | mixed
```

---

## Next Step

After impact analysis, hand off based on the current stage:

```
✓ Impact analysis completed
Next command options:
- /executing-plans
- /subagent-work
- /check
Choose one and run it manually.
```
