# Refactor Overview

> What problem the refactor solves, the from-state and to-state, and the top-level success criteria.

**Status**: Planned
**Depends on**: [Naming and Glossary](../../GLOSSARY.md), [Implementation Readiness Audit](../../analysis/readiness-audit/README.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Roko's current codebase has several subsystems each using their own ad hoc transport shapes — local broadcast channels, typed enums, polling loops, and queue-specific APIs. The refactor replaces all of these with one shared transport model: `Bus`-backed `Pulse` topics. This makes the architecture look like one transport surface instead of many, and unlocks the downstream learning, UX, and ecosystem work that depends on a stable shared runtime contract.

---

## The Problem (From-State)

The current code has several symptoms of the hidden transport problem:

- **Typed ad hoc transport enums** scatter publication concerns across crates rather than centralizing them in a shared Bus abstraction.
- **Polling loops in the TUI** and **broadcast assumptions in Conductor and Learning** are workarounds for the absence of a subscription-oriented transport.
- **Doc/runtime drift**: architecture chapters describe `Pulse` and `Bus` as first-class kernel concepts, but the kernel exports neither. New readers learn a vocabulary that is not yet in the code.
- **Subsystem coupling**: callers import each other's transport types, creating cross-crate dependencies that belong on the Bus boundary instead.

The concrete symptoms from the metrics baseline (see [`success-metrics.md`](success-metrics.md)):

| Signal | Current state |
|---|---|
| Cross-crate subsystem transport imports | Non-zero |
| Polling loops in the TUI | Present |
| Transport-specific coupling in Conductor and Learning | Present |
| Direct subsystem broadcast assumptions | Present |
| `Pulse`, `Bus`, `Topic`, `TopicFilter` in `roko-core` | Absent |

---

## The Solution (To-State)

After Phase C completes, the codebase should:

1. Export `Pulse`, `Bus`, `Topic`, `TopicFilter`, and `Datum` from `roko-core`.
2. Have no subsystem-specific transport enums on migrated call paths.
3. Use `Bus.publish(Pulse)` as the universal publication idiom.
4. Use subscription-driven consumption rather than polling or queue-specific assumptions.
5. Have documentation that matches what the code exports.

Phase D extends this to distributed deployments (chain-backed and mesh-backed Bus backends), but the architecture should be coherent and complete at the end of Phase C without it.

---

## Why the Phased Approach

The work is split into four phases so that:

- **Phase A** (docs) can land before the kernel changes exist. If the kernel slips, the docs still describe the intended shape clearly.
- **Phase B** (kernel addition) is additive — no existing callers break.
- **Phase C** (migration) changes call sites one subsystem at a time, so the workspace can stop after any subsystem migration and remain coherent.
- **Phase D** (distributed backends) only makes sense once in-process Bus semantics are settled. Landing it earlier would force backend-specific compromises into the core model.

There is **no point of no return** in Phases A–C. The plan is explicitly staged so work can pause after any phase without incoherence.

---

## Success Criteria (Top Level)

A full A–C success looks like:

- [ ] Architecture docs consistently use the two-medium / two-fabric framing.
- [ ] `roko-core` exports `Pulse`, `Bus`, `Topic`, `TopicFilter`, `Datum`.
- [ ] Subsystem-specific transport enums are gone from migrated call paths.
- [ ] TUI polling loops are replaced with Bus subscriptions.
- [ ] Conductor, Learning, Orchestration, and Agent paths consume the shared transport model.
- [ ] No cross-crate subsystem transport imports remain on migrated paths.

See [`success-metrics.md`](success-metrics.md) for the detailed per-phase checkpoint criteria and the metric table.

---

## Rollback Summary

| Phase | Rollback method | Runtime effect |
|---|---|---|
| A | Revert the documentation change set | None |
| B | Revert the additive kernel work | None (existing behavior intact) |
| C | Revert the affected subsystem migration only | Contained to that subsystem |
| D | Revert backend-specific crate/module additions | Contained to that backend |

---

## Relationship to Other Docs

- **Phased plan detail**: see each phase file (`01-phase-docs-alignment.md` through `04-phase-chain-mesh-buses.md`).
- **Delivery timeline**: see [`strategy/roadmap/milestone-q1-foundation.md`](../roadmap/milestone-q1-foundation.md).
- **Why this order**: see [`analysis/synergy-map/`](../../analysis/synergy-map/README.md) and [`strategy/refactor-phases/dependencies.md`](dependencies.md).
- **Current implementation state**: see [`analysis/readiness-audit/`](../../analysis/readiness-audit/README.md).
- **Source proposal**: `tmp/refinements/06-refactoring-plan.md` (not yet migrated).
