# Design: `roko prd` — Product Requirements Document Management

## The full loop

```
idea → draft → prd → plan → code → verify → learn
 ↑                                              |
 └──────────────────────────────────────────────┘
```

## Proposed CLI commands

```
roko prd                          # show all PRDs and their status
roko prd idea "description"       # capture a quick idea (append to ideas.md or create draft)
roko prd draft                    # list drafts
roko prd draft new "title"        # create a new draft PRD (interactive, agent-assisted)
roko prd draft edit <slug>        # continue refining a draft (resume session)
roko prd draft promote <slug>     # promote draft → published PRD
roko prd show <slug>              # display a PRD
roko prd consolidate              # agent scans all PRDs, merges duplicates, fills gaps
roko prd plan <slug>              # generate implementation plans/tasks from a PRD
roko prd plan all                 # generate plans from all PRDs that don't have them yet
roko prd status                   # show coverage: which PRDs have plans, which are implemented
```

## File structure

```
.roko/
├── config.toml
├── signals.jsonl
├── memory/
│   └── episodes.jsonl
├── plans/                        # implementation plans (tasks.toml)
│   ├── W01-wire-system-prompts/
│   └── P06-process-management/
└── prd/                          # PRD documents
    ├── ideas.md                  # quick captures, unstructured
    ├── drafts/                   # work-in-progress PRDs
    │   ├── tool-registry-v2.md
    │   └── self-improvement.md
    └── published/                # finalized PRDs
        ├── 00-architecture.md
        ├── 01-orchestration.md
        ├── 02-agents.md
        └── ...
```

## PRD document format

```markdown
---
id: prd-02-agents
title: Agent Architecture
status: published           # draft | published | superseded
version: 1
created: 2026-04-08
updated: 2026-04-08
depends_on: [prd-00-architecture]
crates: [roko-agent, roko-core]
plans_generated: [W01, P06]  # auto-updated when plans are created
coverage: 0.45               # auto-computed: checked tasks / total tasks
tags: [agents, backends, safety]
---

# Agent Architecture

## Overview
...

## Requirements

### REQ-AGENT-01: Claude CLI agent
The system MUST spawn Claude CLI with proper flags...

### REQ-AGENT-02: System prompt injection
The system MUST inject role-specific system prompts...

## Acceptance criteria
- [ ] ClaudeCliAgent passes --tools, --settings, --append-system-prompt
- [ ] Each AgentRole produces a multi-section prompt
- [ ] Safety hooks block git checkout, git switch

## References
- Mori source: apps/mori/src/agent/connection.rs
- Implementation plan: plans/W01-wire-system-prompts/
```

## How each command works

### `roko prd idea "description"`

Simplest possible capture — appends to `.roko/prd/ideas.md` with timestamp.
No agent call needed. Just a local append.

```rust
// In roko-cli/src/prd.rs
fn idea(text: &str) {
    let entry = format!("\n- {} — {}\n", chrono::Local::now().format("%Y-%m-%d %H:%M"), text);
    fs::append(".roko/prd/ideas.md", &entry);
}
```

### `roko prd draft new "title"`

Launches an agent session (Strategist role) that:
1. Reads existing PRDs to understand the project
2. Reads ideas.md for context
3. Asks clarifying questions (interactive)
4. Generates a draft PRD in the format above
5. Saves to `.roko/prd/drafts/<slug>.md`

```bash
# Internally does:
roko run --role strategist \
  --prompt "Create a draft PRD for: <title>. Read existing PRDs in .roko/prd/published/ for context and format."
```

### `roko prd draft edit <slug>`

Resumes a session against an existing draft. The agent reads the draft,
proposes improvements, and updates it.

```bash
roko run --role strategist \
  --resume <session_id> \
  --prompt "Continue refining .roko/prd/drafts/<slug>.md"
```

### `roko prd consolidate`

The most powerful command. An agent:
1. Reads ALL PRDs (published + drafts + ideas)
2. Identifies duplicates, overlaps, gaps
3. Proposes merges (two drafts about the same thing → one)
4. Proposes new PRDs for uncovered areas
5. Updates cross-references (depends_on links)
6. Reports a summary

### `roko prd plan <slug>`

Reads a PRD's requirements and acceptance criteria, then generates
plan directories (plan.md + tasks.toml) under `.roko/plans/`.

This is what `generate-plans.sh` does today, but:
- The input is a structured PRD (not a raw checklist)
- The PRD has typed requirements (REQ-XXX) that map 1:1 to tasks
- The PRD has acceptance criteria that become task acceptance tests
- Coverage is auto-updated in the PRD frontmatter

### `roko prd status`

Reads all PRDs and their linked plans, computes coverage:

```
PRD                    Plans  Tasks  Done  Coverage
─────────────────────  ─────  ─────  ────  ────────
00-architecture        0      0      0     —
02-agents              2      11     3     27%
04-verification        1      7      7     100%
06-neuro               0      0      0     — (Phase 2)
```

## Why this is better than raw checklist files

| Today (roko-progress) | With `roko prd` |
|---|---|
| 1,253 items in one massive checklist | Structured PRDs with typed requirements |
| Stale paths, inconsistent state | Auto-computed coverage, auto-updated links |
| Manual checkbox ticking (often wrong) | Coverage computed from plan/task status |
| No way to capture new ideas | `roko prd idea` for quick capture |
| No iteration on requirements | `roko prd draft edit` for refinement |
| Plans generated separately | `roko prd plan` generates directly from PRD |
| Duplicates across docs | `roko prd consolidate` finds and merges |

## Implementation plan

This is itself a candidate for plan generation:

```bash
roko prd idea "Add roko prd subcommand for PRD lifecycle management"
# ... iterate on the idea ...
roko prd draft new "prd-management"
# ... refine the draft ...
roko prd draft promote prd-management
roko prd plan prd-management
roko plan run
# roko implements its own PRD management system
```

## Phase 1 (minimal, implement first)

1. `roko prd idea` — local append, no agent
2. `roko prd` / `roko prd list` — list all PRDs  
3. `roko prd plan <slug>` — generate plans from a PRD (reuses generate-plans.sh logic)

## Phase 2 (agent-assisted)

4. `roko prd draft new` — agent creates structured draft
5. `roko prd draft edit` — agent refines draft
6. `roko prd draft promote` — move to published

## Phase 3 (intelligent)

7. `roko prd consolidate` — agent-driven merge/dedup
8. `roko prd status` — coverage computation
9. TUI integration — browse PRDs, see coverage, launch edits
