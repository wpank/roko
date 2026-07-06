# Refactor Phases

> The four-phase plan to migrate Roko from its current ad hoc transport surfaces to the
> canonical two-medium (`Engram` + `Pulse`), two-fabric (`Substrate` + `Bus`) kernel model.
> Each phase is independently mergeable and reversible.

**Source**: [`docs/00-architecture/33-refactor-plan-phases.md`](../../_migration/cluster-I-strategy.md)
**Last reviewed**: 2026-04-19

---

## Contents

| # | Page | What it covers | Status |
|---|---|---|---|
| — | [00-overview.md](00-overview.md) | The refactor story: problem, from-state, to-state, success criteria | Reference |
| 01 | [01-phase-docs-alignment.md](01-phase-docs-alignment.md) | Phase A — normalize architecture docs before kernel changes land | Planned |
| 02 | [02-phase-kernel-addition.md](02-phase-kernel-addition.md) | Phase B — add `Pulse`, `Topic`, `TopicFilter`, `Bus`, `Datum` without breaking callers | Planned |
| 03 | [03-phase-subsystem-migration.md](03-phase-subsystem-migration.md) | Phase C — migrate each subsystem's transport to Bus-backed Pulses | Planned |
| 04 | [04-phase-chain-mesh-buses.md](04-phase-chain-mesh-buses.md) | Phase D — add chain-backed and mesh-backed Bus backends (Phase 2+) | Deferred |
| — | [dependencies.md](dependencies.md) | Phase dependency DAG: what must land before what | Reference |
| — | [success-metrics.md](success-metrics.md) | How we know a phase succeeded (checkpoints + metrics) | Reference |
| — | [current-status.md](current-status.md) | Point-in-time snapshot of where we are in the phase sequence | Status |

---

## Suggested reading order

**For a new reader:** `00-overview.md` → `01-phase-docs-alignment.md` → `02-phase-kernel-addition.md` → `03-phase-subsystem-migration.md` → `dependencies.md`

**For an implementer starting Phase A:** `00-overview.md` → `01-phase-docs-alignment.md` → `success-metrics.md` → `current-status.md`

**For a reviewer:** `current-status.md` → `dependencies.md` → the specific phase file

---

## Phase summary

| Phase | Label | Duration | Risk | Merge shape |
|---|---|---|---|---|
| A | Docs Alignment | 1 week | None (doc-only) | Reversible by revert |
| B | Kernel Addition | 2 weeks | Low–moderate (additive) | New types, compatibility shims |
| C | Subsystem Migration | 3–4 weeks | Moderate (call sites change) | One subsystem at a time, compiler-assisted |
| D | Chain & Mesh Buses | Phase 2+ | High (expands transport matrix) | New backends and composition layers |
| **A–C total** | | **~6–7 weeks** | | |

---

## Relationship to the roadmap

Phase A–C are the kernel critical path of **Q1** in the consolidated roadmap. Phase D is **Q5–Q6 optionality**.

See [`strategy/roadmap/milestone-q1-foundation.md`](../roadmap/milestone-q1-foundation.md) for the Q1 scope that contains Phases A–C, and [`strategy/roadmap/beyond-current-quarter.md`](../roadmap/beyond-current-quarter.md) for Q5–Q6.

---

## See also

- [`strategy/roadmap/README.md`](../roadmap/README.md) — the quarter-by-quarter delivery plan
- [`analysis/readiness-audit/`](../../analysis/readiness-audit/README.md) — current implementation state
- [`reference/04-bus/`](../../reference/04-bus/README.md) — Bus specification (target state)
- [`reference/02-pulse/`](../../reference/02-pulse/README.md) — Pulse specification (target state)
