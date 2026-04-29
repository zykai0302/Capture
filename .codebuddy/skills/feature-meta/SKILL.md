---
name: feature-meta
description: |
  Meta-skill for understanding and customizing Uniview Feature - the AI workflow system for Claude Code and Cursor. This skill documents the ORIGINAL Feature system design. When users customize their Feature installation, modifications should be recorded in a project-local `feature-local` skill, NOT in this meta-skill. Use this skill when: (1) understanding Feature architecture, (2) customizing Feature workflows, (3) adding commands/agents/hooks, (4) troubleshooting issues, or (5) adapting Feature to specific projects.
---

# feature Meta-Skill

## Version Compatibility

| Item | Value |
|------|-------|
| **feature CLI Version** | 0.3.0-beta.5 |
| **Skill Last Updated** | 2026-01-31 |
| **Min Claude Code Version** | 1.0.0+ |

> ⚠️ **Version Mismatch Warning**: If your feature CLI version differs from above, some features may not work as documented. Run `feature --version` to check.

---

## Platform Compatibility

### Feature Support Matrix

| Feature | Claude Code | Cursor | OpenCode (Future) |
|---------|-------------|--------|-------------------|
| **Core Systems** | | | |
| Workspace system | ✅ Full | ✅ Full | ✅ Planned |
| Task system | ✅ Full | ✅ Full | ✅ Planned |
| Spec system | ✅ Full | ✅ Full | ✅ Planned |
| Slash commands | ✅ Full | ✅ Full | ⏳ TBD |
| Agent definitions | ✅ Full | ⚠️ Manual | ⏳ TBD |
| **Hook-Dependent Features** | | | |
| SessionStart hook | ✅ Full | ❌ None | ⏳ TBD |
| PreToolUse hook | ✅ Full | ❌ None | ⏳ TBD |
| SubagentStop hook | ✅ Full | ❌ None | ⏳ TBD |
| Auto context injection | ✅ Full | ❌ Manual | ⏳ TBD |
| Ralph Loop | ✅ Full | ❌ None | ⏳ TBD |
| **Multi-Agent/Session** | | | |
| Multi-Agent (current dir) | ✅ Full | ⚠️ Limited | ⏳ TBD |
| Multi-Session (worktrees) | ✅ Full | ❌ None | ⏳ TBD |
| `claude --resume` | ✅ Full | ❌ None | ⏳ TBD |

### Legend

- ✅ **Full**: Feature works as documented
- ⚠️ **Limited/Manual**: Works but requires manual steps
- ❌ **None**: Not supported on this platform
- ⏳ **TBD**: Under consideration for future support

### Platform-Specific Notes

#### Claude Code (Full Support)
All features work as documented. Hooks provide automatic context injection and quality enforcement.

#### Cursor (Partial Support)
- **Works**: Workspace, tasks, specs, commands (read manually)
- **Doesn't work**: Hooks, auto-injection, Ralph Loop, Multi-Session
- **Workaround**: Manually read spec files at session start; no automatic quality gates

#### OpenCode (Future Consideration)
- Need to evaluate OpenCode's extension/hook capabilities
- May require adapter layer for hook compatibility
- Core file-based systems should work once integrated

### Designing for Portability

When customizing feature, consider platform compatibility:

```
┌─────────────────────────────────────────────────────────────┐
│                    PORTABLE (All Platforms)                  │
│  - .feature/workspace/    - .feature/tasks/                 │
│  - .feature/spec/         - ..codebuddycommands/               │
│  - File-based configs     - JSONL context files             │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────▼───────────────────────────────┐
│                  CLAUDE CODE ONLY                            │
│  - ..codebuddyhooks/         - ..codebuddysettings.json hooks     │
│  - SubagentStop control   - Auto context injection          │
│  - Ralph Loop             - Multi-Session worktrees         │
│  - claude CLI features    - --resume, --agent flags         │
└─────────────────────────────────────────────────────────────┘
```

---

## Purpose

This is the **meta-skill** for feature - it documents the original, unmodified feature system. When customizing feature for a specific project, record changes in a **project-local skill** (`feature-local`), keeping this meta-skill as the authoritative reference for vanilla feature.

## Skill Hierarchy

```
~/.codebuddy/skills/
└── feature-meta/              # THIS SKILL - Original feature documentation
                               # ⚠️ DO NOT MODIFY for project-specific changes

project/.codebuddy/skills/
└── feature-local/             # Project-specific customizations
                               # ✅ Record all modifications here
```

**Why this separation?**
- User may have multiple projects with different feature customizations
- Each project's `feature-local` skill tracks ITS OWN modifications
- The meta-skill remains clean as the reference for original feature
- Enables easy upgrades: compare meta-skill with new feature version

---

## Self-Iteration Protocol

When modifying feature for a project, follow this protocol:

### 1. Check for Existing Project Skill

```bash
# Look for project-local skill
ls -la .codebuddy/skills/feature-local/
```

### 2. Create Project Skill if Missing

If no `feature-local` exists, create it:

```bash
mkdir -p .codebuddy/skills/feature-local
```

Then create `.codebuddy/skills/feature-local/SKILL.md`:

```markdown
---
name: feature-local
description: |
  Project-specific feature customizations for [PROJECT_NAME].
  This skill documents modifications made to the vanilla feature system
  in this project. Inherits from feature-meta for base documentation.
---

# feature Local - [PROJECT_NAME]

## Base Version
feature version: X.X.X (from package.json or feature --version)
Date initialized: YYYY-MM-DD

## Customizations

### Commands Added
(none yet)

### Agents Modified
(none yet)

### Hooks Changed
(none yet)

### Specs Customized
(none yet)

### Workflow Changes
(none yet)

---

## Changelog

### YYYY-MM-DD
- Initial setup
```

### 3. Record Every Modification

When making ANY change to feature, update `feature-local/SKILL.md`:

**Example: Adding a new command**
```markdown
### Commands Added

#### /feature:my-command
- **File**: `.codebuddy/commands/feature/my-command.md`
- **Purpose**: [what it does]
- **Added**: 2026-01-31
- **Why**: [reason for adding]
```

**Example: Modifying a hook**
```markdown
### Hooks Changed

#### inject-subagent-context.py
- **Change**: Added support for `my-agent` type
- **Lines modified**: 45-67
- **Date**: 2026-01-31
- **Why**: [reason]
```

### 4. Never Modify Meta-Skill for Project Changes

The `feature-meta` skill should ONLY be updated when:
- feature releases a new version
- Fixing documentation errors in the original
- Adding missing documentation for original features

---

## Architecture Overview

feature transforms AI assistants into structured development partners through **enforced context injection**.

### System Layers

```
┌─────────────────────────────────────────────────────────────────────┐
│                        USER INTERACTION                              │
│  /feature:start  /feature:parallel  /feature:finish-work            │
└─────────────────────────────────┬───────────────────────────────────┘
                                  │
┌─────────────────────────────────▼───────────────────────────────────┐
│                         SKILLS LAYER                                 │
│  .codebuddy/commands/feature/*.md   (slash commands)                   │
│  .codebuddy/agents/*.md             (sub-agent definitions)            │
└─────────────────────────────────┬───────────────────────────────────┘
                                  │
┌─────────────────────────────────▼───────────────────────────────────┐
│                          HOOKS LAYER                                 │
│  SessionStart     → session-start.py (injects workflow + context)   │
│  PreToolUse:Task  → inject-subagent-context.py (spec injection)     │
│  SubagentStop     → ralph-loop.py (quality enforcement)             │
└─────────────────────────────────┬───────────────────────────────────┘
                                  │
┌─────────────────────────────────▼───────────────────────────────────┐
│                       PERSISTENCE LAYER                              │
│  .feature/workspace/  (journals, session history)                   │
│  .feature/tasks/      (task tracking, context files)                │
│  .feature/spec/       (coding guidelines)                           │
└─────────────────────────────────────────────────────────────────────┘
```

### Key Design Principles

| Principle | Description |
|-----------|-------------|
| **Specs Injected, Not Remembered** | Hooks enforce specs - agents always receive context |
| **Read Before Write** | Understand guidelines before writing code |
| **Layered Context** | Only relevant specs load (via JSONL files) |
| **Human Commits** | AI never commits - human validates first |
| **Pure Dispatcher** | Dispatch agent only orchestrates |

---

## Core Components

### 1. Workspace System

Track development progress across sessions with per-developer isolation.

```
.feature/workspace/
├── index.md                    # Global overview
└── {developer}/                # Per-developer
    ├── index.md                # Personal index (@@@auto markers)
    └── journal-N.md            # Session journals (max 2000 lines)
```

**Key files**: `.feature/.developer` (identity), journals (session history)

### 2. Task System

Track work items with phase-based execution.

```
.feature/tasks/{MM-DD-slug-assignee}/
├── task.json           # Metadata, phases, branch
├── prd.md              # Requirements
├── implement.jsonl     # Context for implement agent
├── check.jsonl         # Context for check agent
└── debug.jsonl         # Context for debug agent
```

### 3. Spec System

Maintain coding standards that get injected to agents.

```
.feature/spec/
├── frontend/           # Frontend guidelines
├── backend/            # Backend guidelines
└── guides/             # Thinking guides
```

### 4. Hooks System

Automatically inject context and enforce quality.

| Hook | When | Purpose |
|------|------|---------|
| `SessionStart` | Session begins | Inject workflow, guidelines |
| `PreToolUse:Task` | Before sub-agent | Inject specs via JSONL |
| `SubagentStop:check` | Check agent stops | Enforce verification (Ralph Loop) |

### 5. Agent System

Specialized agents for different phases.

| Agent | Purpose | Restriction |
|-------|---------|-------------|
| `dispatch` | Orchestrate pipeline | Pure dispatcher |
| `plan` | Evaluate requirements | Can reject unclear reqs |
| `research` | Find code patterns | Read-only |
| `implement` | Write code | No git commit |
| `check` | Review and self-fix | Ralph Loop controlled |
| `debug` | Fix issues | Precise fixes only |

### 6. Multi-Agent Pipeline

Run parallel isolated sessions via Git worktrees.

```
plan.py → start.py → Dispatch → implement → check → create-pr
```

---

## Customization Guide

### Adding a Command

1. Create `.codebuddy/commands/feature/my-command.md`
2. Update `feature-local` skill with the change

### Adding an Agent

1. Create `.codebuddy/agents/my-agent.md` with YAML frontmatter
2. Update `inject-subagent-context.py` to handle new agent type
3. Create `my-agent.jsonl` in task directories
4. Update `feature-local` skill

### Modifying Hooks

1. Edit the hook script in `.codebuddy/hooks/`
2. Document the change in `feature-local` skill
3. Note which lines were modified and why

### Extending Specs

1. Create new category in `.feature/spec/my-category/`
2. Add `index.md` and guideline files
3. Reference in JSONL context files
4. Update `feature-local` skill

### Changing Task Workflow

1. Modify `next_action` array in `task.json`
2. Update dispatch or hook scripts as needed
3. Document in `feature-local` skill

---

## Resources

Reference documents are organized by platform compatibility:

```
references/
├── core/              # All Platforms (Claude Code, Cursor, etc.)
├── claude-code/       # Claude Code Only
├── how-to-modify/     # Modification Guides
└── meta/              # Documentation & Templates
```

### `core/` - All Platforms

| Document | Content |
|----------|---------|
| `overview.md` | Core systems introduction |
| `files.md` | All `.feature/` files with purposes |
| `workspace.md` | Workspace system, journals, developer identity |
| `tasks.md` | Task system, directories, JSONL context files |
| `specs.md` | Spec system, guidelines organization |
| `scripts.md` | Platform-independent scripts |

### `claude-code/` - Claude Code Only

| Document | Content |
|----------|---------|
| `overview.md` | Claude Code features introduction |
| `hooks.md` | Hook system, context injection |
| `agents.md` | Agent types, invocation, Task tool |
| `ralph-loop.md` | Quality enforcement mechanism |
| `multi-session.md` | Parallel worktree sessions |
| `worktree-config.md` | worktree.yaml configuration |
| `scripts.md` | Claude Code only scripts |

### `how-to-modify/` - Modification Guides

| Document | Scenario |
|----------|----------|
| `overview.md` | Quick reference for all modifications |
| `add-command.md` | Adding slash commands |
| `add-agent.md` | Adding new agent types |
| `add-spec.md` | Adding spec categories |
| `add-phase.md` | Adding workflow phases |
| `modify-hook.md` | Modifying hook behavior |
| `change-verify.md` | Changing verify commands |

### `meta/` - Documentation

| Document | Content |
|----------|---------|
| `platform-compatibility.md` | Detailed platform support matrix |
| `self-iteration-guide.md` | How to document customizations |
| `feature-local-template.md` | Template for project-local skill |

---

## Quick Reference

### Key Scripts

| Script | Purpose |
|--------|---------|
| `get_context.py` | Get session context |
| `task.py` | Task management |
| `add_session.py` | Record session |
| `multi_agent/start.py` | Start parallel agent |

### Key Paths

| Path | Purpose |
|------|---------|
| `.feature/.developer` | Developer identity |
| `.feature/.current-task` | Active task pointer |
| `.feature/workflow.md` | Main workflow docs |
| `.codebuddy/settings.json` | Hook configuration |

---

## Upgrade Protocol

When upgrading feature to a new version:

1. **Compare** new meta-skill with current
2. **Review** changes in new version
3. **Check** `feature-local` for conflicts
4. **Merge** carefully, preserving customizations
5. **Update** `feature-local` with migration notes

```markdown
## Changelog

### 2026-02-01 - Upgraded to feature X.Y.Z
- Merged new hook behavior from meta-skill
- Kept custom agent `my-agent`
- Updated check.jsonl template
```
