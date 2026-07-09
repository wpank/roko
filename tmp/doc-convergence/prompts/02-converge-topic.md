# Phase 2: Converge Topic — {{TOPIC_ID}}-{{TOPIC_NAME}}

You are producing a single canonical document for the topic **{{TOPIC_ID}}-{{TOPIC_NAME}}** by reading all existing sources and the actual Rust code, then writing a converged spec.

## Context

Roko has 4 disconnected doc layers. Your job is to read all of them for this topic and produce ONE canonical document that replaces them all.

**Vocabulary rule**: Use v2's vocabulary throughout:
- Engram → **Signal** (durable datum)
- Pulse → **Pulse** (ephemeral event)
- Substrate → **Store** (protocol)
- Gate → **Verify** (protocol)
- Scorer → **Score** (protocol)
- Router → **Route** (protocol)
- Composer → **Compose** (protocol)
- Policy → **React** (protocol)
- Block/Module → **Cell** (computation unit)
- Workflow → **Graph** (typed DAG of Cells)

When referring to actual Rust code, use the code's names (e.g., `Engram`, `Gate`) but note the spec name in parentheses.

## Sources to Read

Read these files for this topic. Skip any that don't exist.

### v2 (canonical spec — start here)
- `/Users/will/dev/nunchi/roko/roko/docs/v2/{{TOPIC_ID}}-{{TOPIC_NAME}}.md`

### v1 (depth — absorb what v2 is missing)
{{V1_FILES}}

### v2-depth (partially absorbed — check status)
{{V2_DEPTH_FILES}}

### tmp/prds (implementation detail)
{{TMP_PRD_FILES}}

### Rust code (ground truth for what's implemented)
{{CRATE_PATHS}}

### Additional sources (if relevant)
- `/Users/will/dev/nunchi/roko/roko/tmp/prds/impl/STATUS.md` — honest implementation audit
- `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/MORI-PARITY-CHECKLIST.md` — parity items (grep for this topic)

## Output Structure

Write the converged document to:
`/Users/will/dev/nunchi/roko/roko/tmp/doc-convergence/output/{{TOPIC_ID}}-{{TOPIC_NAME}}.md`

Use this structure:

```markdown
# {{TOPIC_ID}}: {{TOPIC_TITLE}}

> Converged from: v2/{{TOPIC_ID}}.md + [list other sources used]
> Status: {{DONE|PARTIAL|NOT_STARTED|N/A}}
> Primary crates: {{crate list}}

## 1. Overview

[2-3 paragraph summary of what this topic covers, in v2 vocabulary.
Include the "why" — what problem does this solve, what's the design insight.]

## 2. Specification

[The canonical spec for this topic. This is the UNION of:
- v2's architecture and vocabulary (the skeleton)
- v1's depth (detailed algorithms, math, edge cases)
- tmp/prds' operational detail (config knobs, CLI flags, error handling)

Use v2's "Cell/Graph/Protocol" framing. Every concrete system should be
expressed as a composition of the 5 primitives (Signal, Pulse, Cell, Graph, Protocol)
and 4 patterns (Pipeline, Loop, Functor, Space) where applicable.

Include:
- Data structures (with field-level detail)
- Algorithms (with pseudocode or Rust signatures)
- Configuration (TOML keys, env vars)
- CLI commands (if any)
- API routes (if any)
- Error taxonomy (if any)]

## 3. Implementation Status

[What's actually built in the code TODAY. Be honest — read the code, don't just
repeat what docs say. For each major component:]

| Component | Status | Code Location | Notes |
|---|---|---|---|
| ... | DONE/PARTIAL/NOT_STARTED | `crate/path.rs:line` | ... |

### What works end-to-end
[List the actual working code paths, with CLI commands to verify them]

### What's built but not wired
[Code that exists but isn't called from any runtime path]

### What's missing entirely
[Things the spec describes that have no code at all]

## 4. Implementation Plan

[Concrete tasks to get from current state to spec. Ordered by priority.
Each task should be independently assignable to an agent.]

### Critical path (must-do for self-hosting)
- [ ] Task 1: ...
- [ ] Task 2: ...

### Important (significant capability gaps)
- [ ] Task 3: ...

### Nice-to-have (polish, optimization, edge cases)
- [ ] Task 4: ...

## 5. Discoveries

[NEW insights from reading all sources together. Things like:]

### Synergies
[Does this topic connect to other topics in ways none of the individual docs mention?]

### Redundancies
[Are there duplicate implementations or overlapping concerns across crates?]

### Conflicts
[Do the sources disagree? Which version should win and why?]

### Missing features
[Things that SHOULD exist based on the architecture but nobody has specced yet]

### Design questions
[Open questions that need a human decision]

## 6. Source Reconciliation

[For audit trail — map what came from where]

| Section | Primary Source | Additional Sources | Conflicts Resolved |
|---|---|---|---|
| ... | v2/{{TOPIC_ID}} §N | v1/folder/file.md | ... |
```

## Instructions

1. Read the v2 doc FIRST to establish the canonical framing
2. Read all v1 docs for this topic — note what v2 is missing
3. Read v2-depth docs — note absorption status
4. Read tmp/prds coverage — note implementation tasks and status claims
5. Read the actual Rust code — this is ground truth for what's built
6. Write the converged doc following the output structure above
7. Be honest about status — "code exists" != "feature works"
8. Flag every vocabulary mismatch (code name vs spec name)
9. In the Discoveries section, think creatively about what combining all sources reveals
10. Make implementation tasks concrete enough that an agent could execute them without additional context
