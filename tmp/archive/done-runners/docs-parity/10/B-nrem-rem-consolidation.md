# B - NREM Replay, REM Imagination, Consolidation (Docs 02, 03, 04)

This is the clearest "runtime ahead of docs" section after Doc 16. The
crate already ships replay planning, utility scoring, counterfactual
imagination, and creativity modes. The docs mostly need to stop
describing those as planned.

Generated: 2026-04-18

---

## Shipping Now

### B.01 - Replay planner and Mattar-Daw utility scoring

**Status**: DONE

The replay planner is real runtime:

- `DreamReplayMode` ships the four modes in `replay.rs:14-26`
- `DreamReplayPolicy` ships the tuning knobs in `replay.rs:34-60`
- `DreamReplayBatch.utility_score` ships in `replay.rs:63-74`
- `select_replay_episodes()` ships in `replay.rs:76-117`
- `DreamRunner::plan_replay()` exposes it from `runner.rs:552-556`

Doc 16 should stop marking Mattar-Daw-style replay scoring as absent.

### B.02 - REM counterfactual imagination

**Status**: DONE

`imagination.rs` is already a shipping REM-style imagination module:

- `CounterfactualQuery` in `imagination.rs:17-24`
- `CausalModel` in `imagination.rs:44-99`
- `imagine()` in `imagination.rs:118-174`
- `counterfactual_episode()` in `imagination.rs:291-317`

This is a lighter implementation than the most ambitious prose docs
describe, but it is not hypothetical architecture.

### B.03 - Creativity modes

**Status**: DONE

`ImaginationMode` ships the three Boden-inspired modes in
`imagination.rs:26-42`, and `synthesize_hypotheses()` implements them in
`imagination.rs:176-289`.

### B.04 - Shipping consolidation path is simpler than the docs

**Status**: DONE

The live path is:

- dream logic emits `KnowledgeEntry` values,
- `DreamCycle` persists them through `KnowledgeStore`,
- waking systems can filter by tags such as `dream`, `rem`, `counterfactual`.

The large SQLite staging-buffer story is not the shipping contract. The
runtime uses a simpler direct-write plus tagged-entry model.

---

## Shipping Support, Mixed Wiring

### B.05 - Pattern mining, cross-episode consolidation, and playbooks

**Status**: PARTIAL

Supporting infrastructure is present:

- `PatternMiner` in `pattern_discovery.rs:99-245`
- `CrossEpisodeConsolidator` in `pattern_discovery.rs:291-390`
- `k_medoids()` in `hdc_clustering.rs:54-120`
- `PlaybookStore` in `playbook.rs:192-237`

The support stack exists. What stays mixed is how much of it is invoked
directly from the dreams cycle versus elsewhere in the learning runtime.

### B.06 - Confidence ladder / validation loops

**Status**: PARTIAL

Dream-produced entries already carry `KnowledgeKind` and `KnowledgeTier`,
but the full wake-side validation and strengthening loop is still smaller
than the prose docs imply.

---

## Target-State Only

### B.07 - Advanced counterfactual diversity and plausibility stacks

**Status**: TARGET-STATE

DiCE, DPP, FACE, LOF, GIRL-heavy constraint sets, and similar scoring
stacks are not part of the current runtime.

### B.08 - DRL replay variants

**Status**: TARGET-STATE

HER, PER, ERE, and related replay families are not implemented as named
runtime systems. The current replay planner is smaller and sufficient for
the shipped surface.

---

## What To Carry Into The Live Docs

- Docs 02 and 03 should present replay planning, REM counterfactual imagination, and creativity modes as shipping.
- Doc 04 should describe the current tag-based knowledge write path before discussing heavier staging designs.
- Advanced counterfactual diversity and DRL replay sections should be marked informational or future work.
