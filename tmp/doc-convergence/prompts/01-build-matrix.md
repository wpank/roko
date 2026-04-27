# Phase 1: Build the Topic Matrix

You are auditing all documentation and source code for the Roko project to produce a comprehensive topic matrix.

## Context

Roko has specs scattered across 4 disconnected layers:

1. **docs/v1/** (417 files, Apr 12) — Original PRD corpus. Uses old vocabulary: Engram, Gate, Substrate, Scorer, Router, Composer, Policy. 22 topic folders.
2. **docs/v2/** (34 files, Apr 26) — Canonical spec. Uses new vocabulary: Signal, Cell, Graph, Protocol. 28 numbered docs (01-28). "Everything is a Graph of Cells."
3. **docs/v2-depth/** (180 files, Apr 26) — Companion to v2. Deep-dives that absorb v1 content into v2's vocabulary. ~40% absorbed, ~60% pending.
4. **tmp/prds/** (22 files, Apr 21) — Implementation-oriented PRDs (PRD-01 through PRD-10) + implementation plans (IMPL-01 through IMPL-10). 400+ tasks. Uses v1 vocabulary. Not in git.

Additional sources:
- `bardo-backup/prd/` — 359 files of original bardo PRDs (historical reference)
- `bardo-backup/tmp/roko-progress/MORI-PARITY-CHECKLIST.md` — 1,253 parity items
- `~/Downloads/isfr-index-spec-v4.md` — Standalone ISFR institutional spec
- The actual Rust code in `crates/` — uses v1 vocabulary (Engram, Gate, etc.)

## Your Task

Produce a comprehensive topic matrix as a markdown file at:
`/Users/will/dev/nunchi/roko/roko/tmp/doc-convergence/status/MATRIX.md`

### For each topic, determine:

1. **Topic name** (use v2's naming as canonical)
2. **v2 doc** — which `docs/v2/*.md` covers it
3. **v1 docs** — which `docs/v1/**/*.md` files cover it (list all)
4. **v2-depth docs** — which `docs/v2-depth/**/*.md` files cover it, and their absorption status
5. **tmp/prds coverage** — which PRD-NN and IMPL-NN files cover it, with section numbers if applicable
6. **Primary crates** — which `crates/roko-*/` implement this topic
7. **Code status** — from reading the actual code:
   - DONE: fully works end-to-end
   - PARTIAL: some parts wired, some stubbed
   - NOT STARTED: code exists but nothing connected, or no code at all
   - N/A: design-only topic with no expected code
8. **Vocabulary conflicts** — does the code use different names than v2? which names?
9. **Content conflicts** — do the doc sources disagree on design? briefly describe
10. **Depth gap** — is v2 missing substantial detail that v1 or tmp/prds has?
11. **New since v2** — any content in tmp/prds or elsewhere that v2 doesn't cover at all?

### How to discover topics:

Start from the v2 doc list (01-SIGNAL through 28-ROADMAP), but also check for topics that exist ONLY in v1 or tmp/prds:
- v1 has `15-code-intelligence/`, `16-heartbeat/`, `17-lifecycle/` which may not map 1:1 to v2
- tmp/prds has PRD-10 (Dashboard/TUI) which is much more detailed than v2's 20-SURFACES
- tmp/prds has IMPL-10-DEMO (demo sprint) which has no v2 equivalent

### Key paths:

| What | Path |
|---|---|
| v1 index | `/Users/will/dev/nunchi/roko/roko/docs/v1/INDEX.md` |
| v2 index | `/Users/will/dev/nunchi/roko/roko/docs/v2/00-INDEX.md` |
| v2-depth index | `/Users/will/dev/nunchi/roko/roko/docs/v2-depth/INDEX.md` |
| tmp/prds index | `/Users/will/dev/nunchi/roko/roko/tmp/prds/00-INDEX.md` |
| impl status | `/Users/will/dev/nunchi/roko/roko/tmp/prds/impl/STATUS.md` |
| Crates | `/Users/will/dev/nunchi/roko/roko/crates/` |
| Workspace root | `/Users/will/dev/nunchi/roko/roko/` |

### Output format:

```markdown
# Topic Matrix — Doc Convergence Audit

Generated: {date}

## Summary
- Total topics: NN
- Topics with conflicts: NN
- Topics with depth gaps: NN
- Topics missing from v2: NN
- Code status: NN DONE / NN PARTIAL / NN NOT STARTED / NN N/A

## Matrix

### 01-SIGNAL (Signal and Pulse)

| Source | Files | Notes |
|---|---|---|
| v2 | `01-SIGNAL.md` | Canonical. Signal + Pulse + Bus + Store + Demurrage + HDC |
| v1 | `00-architecture/02-engram.md`, `02b-pulse.md`, `04-decay.md`, ... | 8+ files |
| v2-depth | `01-signal/signal-algebra.md` (Absorbed), ... | 3 absorbed, 0 pending |
| tmp/prds | PRD-01 §2-4, PRD-05 §HDC | Uses "Engram" not "Signal" |
| Code | `roko-core/src/types.rs`, `roko-fs/` | PARTIAL — Engram struct exists, no Bus fabric |

**Vocabulary conflicts**: Code says `Engram`, spec says `Signal`. `type Signal = Engram;` alias planned but not in code.
**Content conflicts**: None major — v2 is a clean superset.
**Depth gap**: v1 has detailed decay math (18-decay-tier-matrix.md) not in v2.
**New since v2**: None.

---

[... repeat for all topics ...]

## Unmapped Content

### Content in v1 not covered by any v2 topic
- ...

### Content in tmp/prds not covered by any v2 topic
- ...

### Content in bardo-backup that may be relevant
- ...
```

## Instructions

1. Read all four index files first to understand the full scope
2. For each v2 topic (01-28), find all matching content in v1, v2-depth, and tmp/prds
3. For each topic, grep the crates to determine code status
4. Check for orphaned content (in v1 or tmp/prds but not in any v2 topic)
5. Be thorough — read actual file contents, don't just match by filename
6. Write the complete matrix to the output path
