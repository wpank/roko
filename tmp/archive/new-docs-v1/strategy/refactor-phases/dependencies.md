# Phase Dependency DAG

> Which phase must land before which, and why the order is what it is.

**Last reviewed**: 2026-04-19

---

## The DAG

```
Phase A (Docs Alignment)
    │
    │  Doc vocabulary must match code vocabulary before kernel types land
    ▼
Phase B (Kernel Addition)
    │
    │  Shared Bus/Pulse vocabulary must exist before call sites can migrate
    ▼
Phase C (Subsystem Migration)
    │
    │  In-process Bus semantics must be stable before backends multiply
    ▼
Phase D (Chain & Mesh Buses)          ← Phase 2+
```

This is a strict linear chain with no parallelism between phases. However, **within Phase C**, individual subsystems can migrate in parallel once earlier subsystems in the migration order have completed their shim removal.

---

## Dependency table

| Phase | Immediate prerequisite | Reason |
|---|---|---|
| A | None | Starting point; safe to land before any code work |
| B | A complete | Docs must describe `Pulse`/`Bus` before they are exported from `roko-core`; avoids doc/code divergence |
| C | B complete | `Bus`, `Pulse`, `Topic`, `TopicFilter`, and `Datum` must exist in `roko-core` before call sites can migrate to them |
| D | C stable + Safety Spine + Deployment Hardening | Distributed trust assumptions require the in-process model to be proven stable and safety infrastructure to be in place |

---

## Intra-Phase C ordering

Within Phase C, subsystem migrations follow a specific order to minimize risk:

```
Runtime-facing callers (step 1)
        │
        ▼
Orchestration (step 2)
        │
        ▼
Agent-side streams (step 3)
        │
        ▼
Conductor (step 4)
        │
        ▼
Learning (step 5)
        │
        ▼
TUI and HTTP surfaces (step 6)
```

Steps 4–6 can begin in parallel once step 3 is complete, since Conductor, Learning, and TUI do not depend on each other's Bus migrations. Steps 1–3 are strictly serial because they establish the shared transport surface.

---

## Cross-plan dependencies

Phase C completion also unlocks downstream roadmap milestones:

| Phase C exit → unlocks |
|---|
| Q2 Learning Substrate (HDC fingerprint, demurrage, heuristics, self-learning loops all assume stable Bus + Pulse topics) |
| Q3 Ecosystem and UX (Plugin SPI, StateHub projection, and realtime wire protocol all depend on a stable shared runtime contract) |
| Q4 Safety Spine (audit tooling coherence requires stable Bus-level provenance) |

These are documented in [`strategy/roadmap/dependencies.md`](../roadmap/dependencies.md).

---

## What can run in parallel with phases

| Parallel work | Notes |
|---|---|
| Analysis work (`analysis/`) | Does not depend on the kernel cutover; can proceed at any time |
| Reference docs (`reference/`) | Phase A writes them; they can be further refined during B and C |
| Research (`research/`) | Independent of the phase timeline |
| UX baseline work (CLI parity, TUI polish) | Can proceed, but the Bus-subscription TUI work must wait for Phase C step 6 |
| Testing (`testing/`) | Test strategy doc can be written anytime; new property tests for Bus added in Phase B |

---

## See Also

- [00-overview.md](00-overview.md) — why these phases exist
- [01-phase-docs-alignment.md](01-phase-docs-alignment.md)
- [02-phase-kernel-addition.md](02-phase-kernel-addition.md)
- [03-phase-subsystem-migration.md](03-phase-subsystem-migration.md)
- [04-phase-chain-mesh-buses.md](04-phase-chain-mesh-buses.md)
- [`strategy/roadmap/dependencies.md`](../roadmap/dependencies.md) — milestone dependency graph
