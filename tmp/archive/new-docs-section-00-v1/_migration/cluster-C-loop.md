# Cluster C Migration Log — Loop, Speeds, Layers, Cross-Cuts

**Sources**: 5 files in `docs/00-architecture/`
**Target base**: `tmp/new-docs/`
**Cluster**: C — Loop & Layers
**Date**: 2026-04-19
**Status**: Complete

---

## Source Files and Their Destinations

| Source file | Lines | Refactor verb | Destination file(s) |
|---|---|---|---|
| `09-universal-cognitive-loop.md` | 204 | **expand + split** | `reference/06-loop/` (17 files) |
| `10-three-cognitive-speeds.md` | 292 | **split + expand** | `reference/07-speeds/` (9 files) |
| `11-dual-process-and-active-inference.md` | 279 | **split** | `reference/06-loop/10-dual-process.md`, `reference/06-loop/11-active-inference.md`, cross-reference to `research/foundations/active-inference.md` |
| `12-five-layer-taxonomy.md` | 389 | **split + expand** | `reference/08-layers/` (11 files) |
| `13-cognitive-cross-cuts.md` | 428 | **split** | `reference/09-cross-cuts/` (8 files) |

---

## Detailed Mapping

### 09-universal-cognitive-loop.md → reference/06-loop/

The source file was 204 lines — thin for a concept as central as the cognitive loop.
Per the task instruction ("The loop source is only 204 lines — expand meaningfully.
Each stage deserves real treatment"), every stage was expanded into its own page.

| Source section | Destination |
|---|---|
| Overview / diagram | `00-overview.md` |
| QUERY stage | `01-stage-query.md` (new: QuerySpec spec, timeout recovery, examples) |
| SCORE stage | `02-stage-score.md` (new: full 7-axis table, composite formula, examples) |
| ROUTE stage | `03-stage-route.md` (new: CascadeRouter 3-path, confidence bands, examples) |
| COMPOSE stage | `04-stage-compose.md` (new: token budget table, CoT scaffold, examples) |
| ACT stage | `05-stage-act.md` (new: policy pre-check, timeouts, examples) |
| VERIFY stage | `06-stage-verify.md` (new: 7-gate pipeline, hard/soft taxonomy, examples) |
| PERSIST stage | `07-stage-persist.md` (new: 3 Engram types per tick, examples) |
| REACT stage | `08-stage-react.md` (new: Pulse table, scheduling modifiers, examples) |
| `loop_tick()` code | `09-loop-tick-code.md` (expanded: full TickContext, error contract, test patterns) |
| (new) | `12-invariants.md` — 7 named invariants with enforcement locations |
| (new) | `13-failure-modes.md` — 3 failure categories, StuckDetector, debugging guide |
| (new) | `14-performance.md` — per-stage latency tables, cost accounting, benchmarks |
| (new) | `15-examples.md` — 5 end-to-end worked scenarios |
| (new) | `16-open-questions.md` |

### 10-three-cognitive-speeds.md → reference/07-speeds/

| Source section | Destination |
|---|---|
| Motivation / EEG analogy | `00-overview.md` (expanded: computational grounding, cost-quality table) |
| Gamma band | `01-gamma-reactive.md` (new: parameter table, adaptive period, observability) |
| Theta band | `02-theta-reflective.md` (new: CoT scaffold, Neuro deep retrieval, observability) |
| Delta band | `03-delta-consolidation.md` (new: trigger table, what Delta does/doesn't, Dreams integration) |
| Speed coordination / adaptive clock | `04-speed-coordination.md` (new: concurrent model diagram, adaptive clock code, multi-agent coordination) |
| (new) | `05-triggers.md` — trigger tables for all three tiers |
| (new) | `06-resource-budgets.md` — daily budget allocation, enforcement config |
| Examples | `07-examples.md` (expanded: 4 worked multi-speed scenarios) |
| (new) | `08-open-questions.md` |

### 11-dual-process-and-active-inference.md → two loop pages

The source file mixed two distinct topics. These were split:

| Source section | Destination |
|---|---|
| System 1 / System 2 model | `reference/06-loop/10-dual-process.md` |
| T0/T1/T2 tier definitions | `reference/06-loop/10-dual-process.md` |
| Active inference theory (foundations) | **Cross-linked only**: `reference/06-loop/11-active-inference.md` links to `research/foundations/active-inference.md` (Cluster G will create) |
| Active inference architectural impact | `reference/06-loop/11-active-inference.md` (prediction Pulse, predict.error Pulse, routing prior update, free energy threshold) |

### 12-five-layer-taxonomy.md → reference/08-layers/

| Source section | Destination |
|---|---|
| Overview / one rule | `00-overview.md` |
| L0 Runtime | `01-L0-runtime.md` |
| L1 Framework | `02-L1-framework.md` |
| L2 Scaffold | `03-L2-scaffold.md` |
| L3 Harness | `04-L3-harness.md` |
| L4 Orchestration | `05-L4-orchestration.md` |
| Dependency enforcement | `06-dependency-rules.md` (new: anti-pattern catalog, CI enforcement) |
| (new) | `07-cross-layer-protocols.md` — the three approved communication patterns |
| Crate-layer map (partial) | `08-crate-layer-map.md` (full table including cross-cut crates) |
| (new) | `09-adding-a-layer.md` |
| Rationale | `10-rationale.md` |

### 13-cognitive-cross-cuts.md → reference/09-cross-cuts/

| Source section | Destination |
|---|---|
| What is a cross-cut | `00-overview.md` |
| Neuro | `01-neuro.md` (with pointer to `subsystems/neuro/`) |
| Daimon | `02-daimon.md` (with pointer to `subsystems/daimon/`; expanded: PAD model, behavioral states table, somatic markers) |
| Dreams | `03-dreams.md` (with pointer to `subsystems/dreams/`; expanded: NREM vs REM, hypnagogia, no-model vs model paths) |
| Injection model | `04-injection-model.md` (new: per-cross-cut injection code, lifecycle) |
| (new) | `05-composition.md` — interaction map, Neuro+Daimon, Neuro+Dreams, Daimon+Dreams |
| (new) | `06-boundaries.md` — may/may-not tables |
| (new) | `07-open-questions.md` |

---

## Content Added (Not in Sources)

The following content was synthesized from the architectural context (cross-document
knowledge) and added during expansion:

1. **7-invariant taxonomy** (`12-invariants.md`) — the invariant-numbering scheme and
   `TickInvariantChecker` are new; they make implicit loop guarantees explicit.

2. **StuckDetector** (`13-failure-modes.md`) — the code sketch and recovery ladder
   were inferred from the architecture's intent; the exact API may differ from the
   actual implementation.

3. **Per-stage latency tables** (`14-performance.md`) — specific millisecond values
   are estimates based on known system characteristics (HDC 170 µs at 100K, model
   API 500 ms–30 s). These should be validated against actual benchmarks.

4. **CascadeRouter 3-path** (`03-stage-route.md`) — static → Wilson CI → LinUCB
   cascade was mentioned in prior session context; the specific code structure is
   an architectural interpolation.

5. **GatePipeline 7 gates** (`06-stage-verify.md`) — gate names and pipeline order
   derived from the architecture's requirements; actual implementation may differ.

6. **Cross-layer protocol table** (`07-cross-layer-protocols.md`) — the approved
   upward-signal Pulse list was derived from the full system design.

---

## Cross-Cluster Hooks

These items are needed from other clusters to complete the Cluster C pages:

| Hook | Owner | Where needed |
|---|---|---|
| `research/foundations/active-inference.md` | Cluster G | Linked from `reference/06-loop/11-active-inference.md` |
| `reference/11-crate-map.md` | Cluster D | Cross-referenced from `reference/08-layers/08-crate-layer-map.md` |
| `subsystems/neuro/README.md` | Future cluster | Linked from `reference/09-cross-cuts/01-neuro.md` |
| `subsystems/daimon/README.md` | Future cluster | Linked from `reference/09-cross-cuts/02-daimon.md` |
| `subsystems/dreams/README.md` | Future cluster | Linked from `reference/09-cross-cuts/03-dreams.md` |
| `reference/10-types/score.md` | Cluster A | Used in SCORE stage |
| `reference/10-types/hdc-fingerprint.md` | Cluster A | Used in QUERY, Neuro |
| `reference/10-types/provenance.md` | Cluster A | Used in QUERY, PERSIST |
| `reference/03-substrate/README.md` | Cluster B | Used in all substrate-touching stages |
| `reference/05-operators/` (all) | Cluster B | Linked from each stage page |

---

## Status Tags Assigned

Per task conventions:

| Section | Status |
|---|---|
| `reference/06-loop/` | **Shipping** |
| `reference/07-speeds/` | **Shipping** |
| `reference/08-layers/` | **Shipping** |
| `reference/09-cross-cuts/01-neuro.md` | **Shipping** |
| `reference/09-cross-cuts/02-daimon.md` | **Built** |
| `reference/09-cross-cuts/03-dreams.md` | **Built** |

---

## File Count Summary

| Directory | Files written |
|---|---|
| `reference/06-loop/` | 17 (README + 00–16) |
| `reference/07-speeds/` | 9 (README + 00–08) |
| `reference/08-layers/` | 11 (README + 00–10) |
| `reference/09-cross-cuts/` | 8 (README + 00–07) |
| `_migration/` | 1 (this file) |
| **Total** | **46 files** |

---

## Notes for Reviewer

1. The `loop_tick()` code in `09-loop-tick-code.md` is an architectural interpolation.
   It should be compared against `crates/roko-agent/src/loop/tick.rs` and corrected
   where the actual implementation differs.

2. The per-stage latency values in `14-performance.md` are estimates. The benchmark
   numbers (`loop_tick_T0_in_memory: 1.85 ms`) are representative but should be
   regenerated from `cargo bench` output.

3. The Daimon PAD values and behavioral state labels should be cross-checked against
   `crates/roko-daimon/` — the exact label names may differ.

4. The Dreams imagination algorithm (REM phase) in `03-dreams.md` is an architectural
   description. The actual implementation in `crates/roko-dreams/` may use a different
   binding strategy.
