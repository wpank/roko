# 00-Architecture Parity Analysis

Post-audit refresh of `docs/00-architecture/`.

This pack is a docs-only verification surface for topic `00`. Its job is to keep the parity
materials honest about what the architecture docs can currently claim after the audit. It does
not authorize crate edits, speculative implementation work, or long-horizon roadmap design.

Generated: 2026-04-18
Refreshed: 2026-04-18 (PU00, run `pu-run-20260418-112124`)

---

## Batch Intent

- Keep parity wording aligned with audited codebase reality.
- Split shipped code, partial wiring, and target-state design.
- Mark zero-code concepts as `planned`, `target-state`, or `deferred`.
- Keep carry-forward boundaries explicit when an issue belongs to another topic.
- Maintain `SOURCE-INDEX.md` as a spot-check aid, not as implementation proof.

Hard corrections applied in this refresh:

- Workspace baseline: **36 workspace members**, **322,088 Rust LOC**
- `roko-serve`: **wired**, **200+ routes**
- TUI: **wired**, **~58K LOC**
- Event-bus reality: exactly **two live RokoEvent variants** (`PlanRevision`, `PrdPublished`)
- Engram is the live durable kernel noun; parity materials should use Engram-centered wording and
  treat older naming drift as cleanup residue
- `Pulse`, `Datum`, `Demurrage`, `Worldview`, and `Custody`: **0 lines of code**

Docs covered here: **39 markdown files total** in `docs/00-architecture/`:

- numbered docs `00-35`
- plus `02b`, `07b`, and `INDEX.md`

## Post-Audit Arc Map

| Doc Range | Arc | Audit posture | What matters now |
|-----------|-----|---------------|------------------|
| `00-05` | Foundation | `keep` + `narrow` | Keep Engram, score, decay, provenance in present tense; do not backfill Pulse or demurrage |
| `06-08` | Trait system | `narrow` | Keep the six live traits; defer `Datum` and operator generalization |
| `09-11` | Cognitive loop | `narrow` | Describe `loop_tick()` as shared logic with incomplete runtime ownership |
| `12-17` | Architecture layers | `keep` + `rewrite` | Correct serve/TUI status; split current crate reality from planned crate graph |
| `18-22` | Implementation details | `rewrite` | Correct counts and move demurrage-heavy claims to target-state language |
| `23-29` | Advanced capabilities | `defer` | Preserve useful analysis, but treat most chapters as future-work design |
| `30-35` | Innovation / meta | `rewrite` + `defer` | Treat synergy and roadmap docs as planning artifacts, not current moat proof |

## How To Use This Pack

Treat these files as a verification map for the architecture docs, not as an implementation queue
for `crates/`.

- If a concept is live in code, say so plainly and cite the current surface.
- If a concept is only described in docs, move it into explicit future-work language.
- If a document mixes current state and target state, split those two stories.
- If a later topic owns the real work, keep only the handoff contract here.
- If a sentence cannot be grounded by source docs, code, or audit findings, weaken it.

Default question for every edit:

`What evidence justifies this sentence: live code, partial wiring, or target-state intent?`

This pack is calibrated for a **single-developer-plus-agents** refresh, not a staffed quarterly
program. Keep the parity output small, factual, and explicitly bounded.

## Section Index

| File | Docs Covered | Post-Audit Posture | Notes |
|------|--------------|--------------------|-------|
| [A-foundation.md](A-foundation.md) | 00-05 | `keep` + `narrow` | REF01-03 become disciplined wording changes, not implementation backlog |
| [B-trait-system.md](B-trait-system.md) | 06-08 | `narrow` | Six traits stay central; operator generalization is deferred |
| [C-cognitive-loop.md](C-cognitive-loop.md) | 09-11 | `narrow` | REF05 is a target narrative, not current universal runtime truth |
| [D-architecture-layers.md](D-architecture-layers.md) | 12-17 | `keep` + `rewrite` | Serve and TUI are explicitly wired; proposed crate splits stay proposed |
| [E-implementation-details.md](E-implementation-details.md) | 18-22 | `rewrite` | 36-member / 322K baseline; demurrage remains deferred |
| [F-advanced-capabilities.md](F-advanced-capabilities.md) | 23-29 | `defer` | Pulse, Datum, Worldview, and related concepts stay out of present tense |
| [G-innovation-meta.md](G-innovation-meta.md) | 30-35 | `rewrite` + `defer` | Synergy matrix and roadmap read as planning aids only |
| [BATCHES.md](BATCHES.md) | — | `rewrite` | Narrow docs-only execution contract |
| [SOURCE-INDEX.md](SOURCE-INDEX.md) | — | `rewrite` | Refreshed anchors for docs `23-35` plus code and audit spot checks |

## What Stays Grounded

- Docs `00-03`: Engram-centered kernel framing and the score model are grounded.
- Docs `05-17`: provenance, six traits, three speeds, and large parts of the runtime framing are
  directionally correct.
- Most of the work in this batch is wording discipline and fact correction, not deletion.

## What Needs Narrowing

- A generic `Bus<E>` trait can be discussed as the smallest plausible future transport addition,
  but not as a shipped Pulse fabric.
- `loop_tick()` should be described as a partial shared owner, not as the sole production loop.
- Active inference should be described as partially implemented and partially orphaned, not as the
  dominant live routing mechanism.
- The crate map must clearly split current workspace reality from planned crate splits.

## What Must Stay Deferred

- `Pulse`
- `Datum`
- `Demurrage`
- `Worldview`
- `Custody`
- attention-token economy / VCG
- temporal knowledge topology
- emergent goal structures
- cognitive energy model
- roadmap staffing posture beyond dependency ordering

## What Must Be Rewritten

- `34-synergy-integration-map.md`: the matrix is an aspirational design aid, not moat proof
- `35-consolidated-roadmap.md`: keep dependency ordering; drop staffed quarter-plan posture
- any parity note that still implies `roko-serve` or the TUI are unwired
- any parity note that treats target-state kernel types as current runtime nouns

## Realistic Execution Order

See [BATCHES.md](BATCHES.md) for the execution contract. The short version:

`P1 -> P2 -> P3 -> P4 -> P5`

1. Tighten the parity contract and context pack.
2. Refresh foundation, trait, and loop notes.
3. Correct architecture-layer and implementation-detail facts.
4. Move advanced and meta material into explicit deferred or planning language.
5. Sync source anchors and verification wording, then run text checks.

That order is a narrow editorial pass, not a hidden implementation ladder.

Verification in this batch is textual:

- read source docs
- compare against code and audit notes
- rewrite parity wording
- run text and syntax checks

## Carry-Forward Boundaries

These findings are real, but they are not owned by this docs refresh:

| Item | Better Home | What `00` Should Keep |
|------|-------------|-----------------------|
| generic `Bus<E>` trait | future code parity / architecture cleanup | note that it is the smallest plausible transport addition |
| event enum unification | code parity after docs refresh | state that the live bus still has only two event types |
| safety spine expansion | `11-safety` parity work | split shipped attestation/taint from planned immune-system layers |
| plugin SPI tiers, web UI, gRPC | later topic parity or future planning | mark as deferred |
| long-horizon roadmap | future planning docs | reduce to dependency ordering, not dates or team plans |

## Success Definition

Batch `00` is successful when:

- every parity file under `tmp/docs-parity/00/` distinguishes current state from target state,
- stale legacy naming in parity materials is replaced by Engram-centered wording,
- serve and TUI are explicitly marked as wired,
- zero-code concepts are labeled `planned` or `deferred`,
- docs `30-35` read as planning artifacts with dependency ordering rather than as a live 5-7
  engineer execution plan,
- and `SOURCE-INDEX.md` helps editors verify claims instead of implying implementation proof.
