# Batch Execution Contract

Run: PU00 (`pu-run-20260418-130603`)

This file defines the post-audit execution contract for `tmp/docs-parity/00/`.

These are **docs-only** batches. They refresh parity materials so they match the audit and the
current codebase reality. They do **not** implement the architecture inside `crates/`, and they do
not convert speculative designs into implied commitments.

---

## Batch Posture

- Ship documentation truth, not speculative architecture.
- Prefer `keep`, `narrow`, `defer`, and `rewrite` over fake parity.
- Close a batch when wording is truer and the pack is internally consistent.
- Verify with text and syntax checks, not `cargo` commands.
- If a concept has zero code, do not describe it as existing.
- If a fix would require code outside `tmp/docs-parity/00/`, carry it forward instead.
- Calibrate every task for a single-developer-plus-agents pass, not a staffed quarter plan.
- Keep each batch finishable in a focused 90-minute docs pass.

Required reads for every batch:

- [00-INDEX.md](00-INDEX.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md)
- [context-pack/agent-runbook.md](context-pack/agent-runbook.md)
- [context-pack/architecture-summary.md](context-pack/architecture-summary.md)
- [context-pack/gaps-summary.md](context-pack/gaps-summary.md)
- [context-pack/carry-forward-map.md](context-pack/carry-forward-map.md)

## Recommended Serial Order

`P1 -> P2 -> P3 -> P4 -> P5`

This order moves from the contract and shared facts, to the grounded docs, to stale-fact
corrections, to speculative sections, then finishes with source-index verification cleanup.

## Batch Overview

| Batch | Time Box | Purpose | Primary Files | Verify Focus |
|-------|----------|---------|---------------|--------------|
| P1 | 15-20 min | Tighten the parity contract and context pack | `00-INDEX.md`, `BATCHES.md`, `context-pack/*.md` | counts/status consistent; docs-only scope explicit; Engram-centered wording; 90-minute scope explicit |
| P2 | 15-20 min | Refresh foundation, trait, and loop analyses | `A-foundation.md`, `B-trait-system.md`, `C-cognitive-loop.md` | REF01-05 narrowed; Pulse and Datum remain planned |
| P3 | 10-15 min | Correct architecture-layer status and implementation-scale facts | `D-architecture-layers.md`, `E-implementation-details.md` | serve/TUI wired; 36 members / 322,088 LOC |
| P4 | 10-15 min | Rewrite advanced and meta docs as deferred research or planning material | `F-advanced-capabilities.md`, `G-innovation-meta.md` | docs `23-35` stop reading like present-tense implementation proof |
| P5 | 10-15 min | Refresh source anchors and runner wording | `SOURCE-INDEX.md`, `run-docs-parity.sh` | docs `23-35` anchors current; runner matches narrowed docs-only contract |

## Batch Details

### P1 — Contract And Context

**Owns**:

- `00-INDEX.md`
- `BATCHES.md`
- `context-pack/agent-runbook.md`
- `context-pack/architecture-summary.md`
- `context-pack/gaps-summary.md`
- `context-pack/carry-forward-map.md`
- `context-pack/repo-map.md`

**Goal**:

Turn the parity pack into an audit-aligned verification brief:

- docs-only scope
- current-vs-planned discipline
- corrected counts and wiring status
- explicit carry-forward boundaries
- Engram-centered wording instead of stale legacy naming

**Out of scope**:

- crate edits
- new implementation tasks
- roadmap design longer than a short future-work note

**Verify**:

```bash
rg -n "36 workspace members|32 crates \\+ 3 apps \\+ 1 test crate|322,088 Rust LOC|200\\+ routes|58K LOC|two live RokoEvent variants|Engram" \
  tmp/docs-parity/00/00-INDEX.md \
  tmp/docs-parity/00/BATCHES.md \
  tmp/docs-parity/00/context-pack/*.md
```

### P2 — Grounded Architecture Docs

**Owns**:

- `A-foundation.md`
- `B-trait-system.md`
- `C-cognitive-loop.md`

**Goal**:

Preserve the useful diagnosis while narrowing the prescription:

- Engram remains the live durable kernel noun
- Pulse and Datum become planned or deferred
- Bus becomes a possible future trait, not a shipped transport rewrite
- loop and active-inference claims become partial, not absolute
- the live transport stays described as exactly two live `RokoEvent` variants, not as an implied
  Pulse fabric

**Out of scope**:

- proving parity by implementing missing runtime wiring
- generalized operators across all traits
- target-state buses, pulses, or datum surfaces

**Verify**:

```bash
rg -n "planned|deferred|target-state|target narrative|exactly two live RokoEvent variants" \
  tmp/docs-parity/00/A-foundation.md \
  tmp/docs-parity/00/B-trait-system.md \
  tmp/docs-parity/00/C-cognitive-loop.md
! rg -n "Pulse is the live|Datum is the current contract|Bus is the shipped seventh trait" \
  tmp/docs-parity/00/A-foundation.md \
  tmp/docs-parity/00/B-trait-system.md \
  tmp/docs-parity/00/C-cognitive-loop.md
```

### P3 — Runtime Reality Corrections

**Owns**:

- `D-architecture-layers.md`
- `E-implementation-details.md`

**Goal**:

Correct stale facts and split current implementation from aspirational docs:

- serve and TUI are wired
- workspace baseline is 36 workspace members / 322,088 Rust LOC
- planned crate graph and demurrage-heavy models stay clearly future-state

**Out of scope**:

- architectural crate splits
- demurrage implementation
- compound kind implementation

**Verify**:

```bash
rg -n "36 workspace members|322,088 Rust LOC|200\\+ routes|58K LOC|wired" \
  tmp/docs-parity/00/D-architecture-layers.md \
  tmp/docs-parity/00/E-implementation-details.md
! rg -n "HTTP API not[[:space:]]wired|Text-mode dashboard[[:space:]]only|177[Kk]|18\\+[[:space:]]crates" \
  tmp/docs-parity/00/D-architecture-layers.md \
  tmp/docs-parity/00/E-implementation-details.md
```

### P4 — Deferred And Meta Docs

**Owns**:

- `F-advanced-capabilities.md`
- `G-innovation-meta.md`

**Goal**:

Move research and planning content out of present-tense architecture claims:

- docs `23-29` become explicit future work
- docs `30-35` become planning/reference material
- synergy matrix is called aspirational fiction
- roadmap is reduced to dependency ordering, not staffed quarter planning
- planning artifact language stays explicit across both files

**Out of scope**:

- defending speculative primitives as if parity merely needs more effort
- backfilling future work into a batch-00 implementation queue

**Verify**:

```bash
rg -n "aspirational fiction|planning artifact|dependency ordering|single-developer-plus-agents|deferred" \
  tmp/docs-parity/00/F-advanced-capabilities.md \
  tmp/docs-parity/00/G-innovation-meta.md
```

### P5 — Source Index Verification Pass

**Owns**:

- `SOURCE-INDEX.md`
- `run-docs-parity.sh`

**Goal**:

Make the pack easier to verify:

- refresh stale anchors for docs `23-35`
- describe anchors as spot checks, not proof of implementation
- keep the runner wording aligned with the narrowed docs-only contract

**Out of scope**:

- new code anchors for zero-code concepts

**Verify**:

```bash
bash -n tmp/docs-parity/00/run-docs-parity.sh
rg -n "verification, not evidence|spot-check anchors|23-architectural-analysis-improvements.*34|24-cross-section-integration-map.*165|30-cross-pollination-innovations.*34|34-synergy-integration-map.*25|35-consolidated-roadmap.*40" \
  tmp/docs-parity/00/SOURCE-INDEX.md
```

## PU00 Scope Boundary

This batch (`PU00`) covers only `tmp/docs-parity/00/`. Later parity batches cover other topics:

- `PU01+`: safety, learning, serving, and composition parity
- `PE*` batches: crate-level implementation work that the docs parity surfaces

Work that belongs to `PE*` should never be pulled into `PU00`.
