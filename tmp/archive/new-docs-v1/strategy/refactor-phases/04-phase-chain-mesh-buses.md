# Phase D — Chain & Mesh Buses

> Add chain-backed and mesh-backed Bus backends, and a multi-backend composition layer, for Phase 2+ distributed deployment shapes.

**Status**: Deferred (Phase 2+)
**Phase index**: 04 of 04
**Duration estimate**: TBD (Phase 2+ budget)
**Risk**: High (expands the transport matrix; distributed trust assumptions)
**Merge shape**: New backends and composition layers; isolated from core
**Depends on**: [Phase C — Subsystem Migration](03-phase-subsystem-migration.md), Safety Spine (Q4), Deployment Hardening (Q4)
**Unlocks**: Cross-deployment witness flows, ChainBus replay, multi-process agent meshes
**Last reviewed**: 2026-04-19

---

## Goal

Extend the in-process Bus model established in Phases B and C to distributed deployment shapes. By the end of Phase D, Roko supports on-chain replay and multi-process mesh deployments without forcing backend-specific details into the core `Bus` trait or into callers.

---

## Why It Is Later

The additional backends only make sense once the in-process Bus semantics are settled. Landing them in Phase 1 would force backend-specific compromises into the core model. Phase D is intentionally deferred so that:

1. The `Bus` trait interface is proven stable before backends multiply.
2. Safety infrastructure (custody, taint, provenance) is in place before distributed trust flows are added.
3. Deployment hardening (multi-tenant shape, identity integration) is available before cross-deployment witness flows land.

---

## Scope

### Core additions

| Item | Description |
|---|---|
| `ChainBus` | A Bus backend backed by on-chain storage for replay and auditability |
| `MeshBus` | A Bus backend for multi-process and distributed deployment shapes |
| `MultiBus` | A composition layer that fans out or routes across multiple Bus backends |
| Replay semantics | Formal definition and tests for what replay means per backend |

### What does NOT change

- The `Bus` trait interface from Phase B.
- The in-process `roko-std` Bus implementations.
- The Pulse, Topic, and TopicFilter vocabulary.
- Any migrated subsystem's publication or subscription logic.

---

## Prerequisites

- Phase C complete and stable (all subsystems migrated, no shim debt).
- Safety spine landed (Q4): custody records, provenance, taint tracking, audit tooling.
- Deployment hardening landed (Q4): multi-tenant shape, identity integration.
- The `Bus` trait interface has been stable for at least one quarter without interface changes.

---

## Deliverables

1. `ChainBus` backend in its own crate, with replay behavior defined and tested.
2. `MeshBus` backend for multi-process deployment, with parity tests against the in-process Bus.
3. `MultiBus` composition layer for routing across backends.
4. Documentation for each backend's replay semantics and behavioral differences from the in-process Bus.
5. Migration guide for existing deployments adopting distributed backends.

---

## Exit Criteria

- [ ] Chain and mesh backends have parity with the core `Bus` surface (same `Topic`/`TopicFilter` vocabulary).
- [ ] Replay behavior is defined and tested for each backend.
- [ ] Multi-backend composition works without forcing backend-specific details into callers.
- [ ] Existing in-process deployments can add a distributed backend without changing their publication or subscription code.

---

## Current Status

Deferred. Not on the critical path for Q1–Q4. Deliberate sequencing, not deferral by accident — Q1–Q4 should produce a coherent product even if Phase D never lands.

See [`strategy/roadmap/beyond-current-quarter.md`](../roadmap/beyond-current-quarter.md) for the Q5–Q6 context.

---

## Roadmap Alignment

Phase D corresponds to the **Q5–Q6 Phase 2 Optionality** section of the consolidated roadmap. It aligns with REF09 (Phase-2 Bus/Substrate backends). See [`strategy/roadmap/milestone-q5q6-phase2-optionality.md`](../roadmap/milestone-q5q6-phase2-optionality.md).

---

## Risks

1. **Backend schema drift** — chain and in-process semantics may diverge. Mitigation: formal replay spec written before implementation; parity test suite.
2. **Distributed trust assumptions** may not be compatible with the core `Bus` model if the core model was not designed with them in mind. Mitigation: the safety spine (Q4) must land first.
3. **Interface instability** — if the `Bus` trait is still evolving, backends will break. Mitigation: enforce a freeze on the `Bus` interface boundary before Phase D begins.

---

## See Also

- [Phase C — Subsystem Migration](03-phase-subsystem-migration.md)
- [dependencies.md](dependencies.md)
- [`strategy/roadmap/milestone-q5q6-phase2-optionality.md`](../roadmap/milestone-q5q6-phase2-optionality.md)
- [`reference/04-bus/`](../../reference/04-bus/README.md) — Bus trait specification
