# Workflow Context

This document provides shared context for all Feature workflow commands.

## Directory Structure

```text
.feature/
├── tasks/
│   └── <task>/
│       ├── prd.md                # Product Requirements Document
│       ├── task_plan.md          # Phase tracking: where am I?
│       ├── findings.md           # Key discoveries: what have I learned?
│       ├── progress.md           # Execution log: what have I done?
│       ├── implementation-plan.md # Step-by-step implementation plan
│       ├── testcase.md           # Test case specifications
│       ├── testcase_checkdetail.md # Test case check details
│       └── task.json             # Task metadata (status, executionMode)
├── spec/
│   ├── backend/                  # Backend coding standards
│   ├── frontend/                 # Frontend coding standards
│   └── guides/                   # Thinking checklists
├── solutions/                    # Captured solutions
│   └── <category>/
│       └── <problem-slug>-<date>.md
└── workspace/
    └── <developer>/
        ├── index.md              # Journal index
        └── journal-N.md          # Session records
```

## Task Memory Files

Three core files that persist task context:

1. **task_plan.md** - Answers "where am I?"
   - Current phase
   - Phase history
   - Next steps

2. **findings.md** - Answers "what have I learned?"
   - Constraints
   - Decisions
   - Risks
   - Key discoveries

3. **progress.md** - Answers "what have I done?"
   - Commands run
   - Files modified
   - Test results
   - Blockers encountered

## Workflow Phases

1. **start** - Task initialization and classification
2. **discovery** - Requirement clarification
3. **planning** - Memory setup and research
4. **development** - Implementation
5. **verification** - Testing and validation
6. **completion** - Review and knowledge capture

## Core Rules

- Read before decide: Always read task memory files before making decisions
- Update after action: Update task memory after each meaningful action
- Plan before code: Never skip planning phase
- Verify before commit: Always run verification before committing

## GitNexus Integration

When GitNexus is available:
- Use `mcp__gitnexus__impact` for blast radius analysis
- Use `mcp__gitnexus__detect_changes` for diff-based impact
- Refresh index when stale

## Scripts

- `get_context.py` - Get current task context
- `task.py` - Task management (list, archive)
- `add_session.py` - Add session to journal
