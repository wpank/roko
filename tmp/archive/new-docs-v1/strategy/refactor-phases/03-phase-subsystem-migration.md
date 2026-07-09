# Phase C — Subsystem Migration

> Replace subsystem-specific transport enums and broadcast assumptions with Pulse topics and Bus subscriptions across all call sites, one subsystem at a time.

**Status**: Planned
**Phase index**: 03 of 04
**Duration estimate**: 3–4 weeks
**Risk**: Moderate (call sites change; runtime behavior may shift)
**Merge shape**: One subsystem at a time, compiler-assisted, independently mergeable
**Depends on**: [Phase B — Kernel Addition](02-phase-kernel-addition.md)
**Unlocks**: [Phase D — Chain & Mesh Buses](04-phase-chain-mesh-buses.md), and the entire Q2 learning substrate
**Last reviewed**: 2026-04-19

---

## Goal

Remove the hidden transport model scattered across the workspace. After Phase C, the architecture looks like one transport surface with named topics rather than several incompatible transport surfaces.

This is the first phase that changes runtime call sites. The migration is compiler-assisted (old APIs become errors, guiding engineers to the new path) and can be staged one subsystem at a time.

---

## Scope

Phase C replaces subsystem-specific transport enums and broadcast assumptions with Pulse topics and Bus subscriptions. The point is not only to rename; it is to remove transport coupling between subsystems.

### What changes

| Before | After |
|---|---|
| Typed ad hoc transport enums | Topic names + typed payloads |
| `local_broadcast(msg)` | `Bus.publish(Pulse { topic, payload })` |
| Queue-specific consumption | Subscription-driven, topic-filtered |
| Polling loops (TUI) | Bus subscriptions |
| Cross-crate transport type imports | Shared Bus vocabulary only |

### Migration order

The migration order is chosen to minimize risk: runtime-facing call sites move first (widest shared surface), then subsystems that depend on them.

| Step | Subsystem | Rationale |
|---|---|---|
| 1 | **Runtime-facing callers** | The shared transport surface must become Bus-shaped before subsystems start subscribing |
| 2 | **Orchestration** | Plan-driven publication is easy to topic-encode; early win to prove the model |
| 3 | **Agent-side streams** | Agent-to-agent and sidecar publication paths follow the shared surface |
| 4 | **Conductor** | Migrates to Bus subscriptions; sheds its dependency shape where the plan allows |
| 5 | **Learning** | Prediction and outcome topics become topic-driven |
| 6 | **TUI and HTTP surfaces** | Polling and local broadcast assumptions replaced by Bus subscriptions |

---

## Prerequisites

- Phase B must be complete: `Pulse`, `Bus`, `Topic`, `TopicFilter`, `Datum`, and the in-process Bus must be exported from `roko-core`.
- Compatibility shims from Phase B must still be in place so non-migrated callers continue to compile throughout the migration window.

---

## Deliverables

For each subsystem in the migration order:

1. Ad hoc transport enums removed from the migrated call paths.
2. Publication uses `Bus.publish(Pulse { topic, payload })`.
3. Consumption is subscription-driven.
4. Compatibility shims for that subsystem removed.
5. Tests confirming the migrated paths behave correctly.

Across the full phase:

- Cross-crate subsystem transport imports reduced to zero on migrated paths.
- TUI polling loops eliminated; replaced with Bus subscriptions.
- Conductor, Learning, Orchestration, and Agent paths all consume the shared transport model.

---

## Exit Criteria

- [ ] Subsystem-specific transport enums are gone from all migrated call paths.
- [ ] Publishing uses Pulse topics rather than local broadcast assumptions.
- [ ] TUI, Conductor, Learning, and Orchestration paths all consume the shared transport model.
- [ ] No cross-crate subsystem transport imports remain on migrated paths.
- [ ] All deprecation shims introduced in Phase B have been removed.

---

## Current Status

Not started. Depends on Phase B reaching exit criteria.

---

## Roadmap Alignment

Phase C is the subsystem migration portion of the **Q1 Foundation** milestone. The Q2 Learning Substrate milestone assumes Phase C is complete (HDC fingerprint, demurrage, heuristics, and self-learning loops all rely on stable Pulse topics and Bus subscriptions). See [`strategy/roadmap/milestone-q1-foundation.md`](../roadmap/milestone-q1-foundation.md) and [`strategy/roadmap/milestone-q2-learning-substrate.md`](../roadmap/milestone-q2-learning-substrate.md).

---

## Risks

1. **Migration churn** if several subsystems try to change their transport shapes simultaneously. Mitigation: strict serial order within Phase C; no subsystem starts migrating until the previous one's shims are removed.
2. **Backend schema drift** if two subsystems converge on incompatible topic naming conventions. Mitigation: a topic naming spec (part of Phase A docs deliverables) enforced before any subsystem migration begins.
3. **Regression in graduation policy** — a Pulse that should have been graduated to Engram isn't. Mitigation: property tests for the graduation path added in Phase B.

---

## See Also

- [Phase B — Kernel Addition](02-phase-kernel-addition.md)
- [Phase D — Chain & Mesh Buses](04-phase-chain-mesh-buses.md)
- [dependencies.md](dependencies.md)
- [success-metrics.md](success-metrics.md)
- [`strategy/roadmap/milestone-q2-learning-substrate.md`](../roadmap/milestone-q2-learning-substrate.md) — the milestone unlocked by this phase
