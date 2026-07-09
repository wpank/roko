# Batch Execution Contract

Narrow docs-refresh batches for `tmp/docs-parity/07/`.

Generated: 2026-04-18

---

## Batch Posture

- This is a **parity refresh**, not a runtime implementation plan.
- Stay inside `tmp/docs-parity/07/`.
- Prefer short, source-backed corrections over new architecture.
- Mark theory and Phase 2+ work explicitly instead of deleting it.
- Verification is lightweight: grep for refreshed wording and run `bash -n`
  on the runner script.

---

## Recommended Order

`C1 -> C2 -> C3 -> C4 -> C5 -> C6`

This order first fixes the top-level framing, then the shipped runtime docs,
then the theory/frontier labeling, and finally the operator support files.

---

## Batch Overview

| Batch | Purpose | Primary files | Verify focus | Est. time |
|-------|---------|---------------|--------------|-----------|
| C1 | Reset section posture and architecture claims | `00-INDEX.md`, `A-architecture.md` | `rg -n "10 watchers|RoutingBias|Bus<E>|Phase 2\\+" tmp/docs-parity/07/00-INDEX.md tmp/docs-parity/07/A-architecture.md` | 15 min |
| C2 | Refresh watcher and decision docs around what is actually live | `B-watchers-signals.md`, `C-decision-space.md` | `rg -n "conductor:alert|conductor.decision|CognitiveSignal|Continue \\| Restart \\| Fail" tmp/docs-parity/07/B-watchers-signals.md tmp/docs-parity/07/C-decision-space.md` | 15 min |
| C3 | Refresh diagnosis, stuck, health, and process-support status | `D-diagnosis-stuck.md`, `E-health-adaptive.md` | `rg -n "34 built-in patterns|6 heuristics|HealthMonitor|ProcessSupervisor|ownership split" tmp/docs-parity/07/D-diagnosis-stuck.md tmp/docs-parity/07/E-health-adaptive.md` | 15 min |
| C4 | Mark theory and learning chapters as informational or Phase 2+ | `F-theory-learning.md` | `rg -n "informational|Phase 2\\+|ConductorBandit|Yerkes-Dodson|Good Regulator" tmp/docs-parity/07/F-theory-learning.md` | 10 min |
| C5 | Refresh source anchors and operator context | `SOURCE-INDEX.md`, `context-pack/*` | `rg -n "conductor\\.rs:82-99|event_bus\\.rs:101-130|workspace members: 36|322,088" tmp/docs-parity/07/SOURCE-INDEX.md tmp/docs-parity/07/context-pack` | 20 min |
| C6 | Append refresh audit note and update the runner script | `AUDIT-LOG.md`, `run-docs-parity.sh` | `bash -n tmp/docs-parity/07/run-docs-parity.sh` | 10 min |

---

## Batch Details

### C1 — Posture Reset

**Goal**: Replace the old "runtime hardening backlog" framing with a
docs-only parity brief.

**Do**:

- state clearly that the conductor core is shipped,
- surface the real remaining seam: `roko-conductor -> roko-learn`,
- keep follow-up items as deferred work, not present-tense architecture.

**Do not**:

- prescribe code changes across crates,
- re-open the Pulse / Datum / demurrage family of concepts.

### C2 — Watchers And Decisions

**Goal**: Make the live watcher and intervention contract easy to read.

**Do**:

- say all 10 watchers are implemented,
- use the real emitted kind names,
- mark `CognitiveSignal` as planned rather than built.

**Do not**:

- turn CEP composition or advanced anomaly fusion into current scope.

### C3 — Diagnosis And Support Modules

**Goal**: Describe diagnosis, stuck detection, health, and process support in
the right tense.

**Do**:

- call out diagnosis as shipped,
- call out stuck detection and health monitoring as implemented support
  surfaces,
- keep the process ownership split visible.

**Do not**:

- blur "implemented module" and "dominant hot-path contract" into the same
  status label.

### C4 — Theory Reclassification

**Goal**: Reframe the theory-heavy chapters as framing or future work.

**Do**:

- keep OODA, Good Regulator, and Yerkes-Dodson as useful context,
- mark federation, self-healing, and multi-level conductor as Phase 2+,
- note that `ConductorBandit` is live only in the retry path.

**Do not**:

- describe theory sections as current engineering requirements.

### C5 — Source And Context Refresh

**Goal**: Give later agents a short, accurate set of anchors and operator
notes.

**Do**:

- refresh key code anchors,
- update workspace numbers,
- make the context-pack reflect the narrowed scope.

### C6 — Audit Trail And Runner

**Goal**: Leave a clean audit note and a runner script that describes this
docs-refresh workflow.

**Do**:

- append a short refresh summary to the audit log,
- swap out cargo-heavy verify commands for docs-refresh checks,
- keep `bash -n` passing.

---

## Completion Standard

A run is complete when:

- every required file under `tmp/docs-parity/07/` has been updated,
- `bash -n tmp/docs-parity/07/run-docs-parity.sh` passes,
- and the conductor docs now distinguish shipped runtime, implemented support
  modules, and Phase 2+/informational theory.
