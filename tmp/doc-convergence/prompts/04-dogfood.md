# Phase 4: Dogfood into Roko

You are converting the converged spec and implementation plans into roko's own PRD and task system, so roko can track its own development.

## Context

Phase 2 produced converged topic docs in `/Users/will/dev/nunchi/roko/roko/tmp/doc-convergence/output/`.
Phase 3 produced a synthesis with revised roadmap in `output/00-SYNTHESIS.md`.

Each converged doc has a "§4. Implementation Plan" section with concrete tasks organized by priority (critical/important/nice-to-have).

Roko's own PRD system uses:
- `.roko/prd/ideas.md` — captured work items
- `.roko/prd/drafts/*.md` — PRD drafts (markdown)
- `.roko/prd/published/*.md` — published PRDs (trigger plan generation)
- `plans/*/tasks.toml` — TOML task files consumed by `roko plan run`

## Your Task

### 1. Create PRD drafts from the synthesis roadmap

For each phase in the synthesis roadmap, create a PRD draft at:
`.roko/prd/drafts/{slug}.md`

Each PRD should follow roko's format:
```markdown
---
slug: {slug}
title: {title}
status: draft
created: {date}
---

# {title}

## Problem
[What gap or issue this addresses]

## Solution
[What to build, referencing converged spec sections]

## Tasks
[High-level task list — will be expanded into tasks.toml]

## Acceptance Criteria
[How to verify this is done]

## References
- Converged spec: `tmp/doc-convergence/output/{topic}.md` §N
- Source docs: [list]
```

### 2. Create task files from implementation plans

For each phase that has concrete tasks, create a TOML task file at:
`plans/{slug}/tasks.toml`

Use roko's task format:
```toml
[metadata]
name = "{plan name}"
description = "{what this plan does}"
created = "{date}"

[[tasks]]
id = "T1"
name = "{task name}"
description = "{detailed description an agent can execute}"
depends_on = []  # task IDs this depends on
role = "engineer"  # engineer|researcher|reviewer
effort = "medium"  # small|medium|large
```

### 3. Update ideas.md

Append any design questions from the synthesis (§7) to `.roko/prd/ideas.md` as captured work items.

## Sources

Read:
- All files in `/Users/will/dev/nunchi/roko/roko/tmp/doc-convergence/output/`
- `/Users/will/dev/nunchi/roko/roko/tmp/doc-convergence/output/00-SYNTHESIS.md` (especially §6 roadmap)
- `/Users/will/dev/nunchi/roko/roko/.roko/prd/ideas.md` (existing ideas, don't duplicate)

## Output

Write a summary of what was created to:
`/Users/will/dev/nunchi/roko/roko/tmp/doc-convergence/status/DOGFOOD-REPORT.md`

## Instructions

1. Read the synthesis roadmap first to understand priority ordering
2. Group tasks into logical PRDs (don't create one PRD per topic — group by phase/theme)
3. Task descriptions must be specific enough for `roko plan run` agents to execute
4. Include file paths and code references in task descriptions
5. Set up task dependencies correctly (what blocks what)
6. Keep task count manageable — aim for 5-15 tasks per plan, not 50+
7. Use the synthesis dependency graph to order phases
