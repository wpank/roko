# Milestone: Q5–Q6 — Phase 2 Optionality

> Distributed backends, Dreams consolidation loop, Composer rewrite, and plugin registry — none required for the architecture to stand.

**Target**: Q5–Q6 (full-team estimate; highly variable with 1 dev + AI agents)
**Status**: Deferred — deliberate sequencing, not deferral by accident
**Owner**: TBD
**Prerequisites**: [Q4 — Scale, Safety, and Domains](milestone-q4-scale-safety-domains.md) complete; Phase D prerequisites (Bus trait frozen, safety spine operational, deployment hardened)
**Unlocks**: Cross-deployment witness flows, ChainBus replay, plugin registry, Dreams online
**Roadmap quarter risk**: High (distributed trust assumptions)
**Last reviewed**: 2026-04-19

---

## Headline

Q5–Q6 are where Phase-2 backends and long-horizon compounding layers become worth landing. Q1–Q4 should produce a coherent product even if Q5–Q6 slips indefinitely.

---

## Scope

| Item | Description | Refactor phase |
|---|---|---|
| ChainBus | On-chain Bus backend with replay | Phase D |
| MeshBus | Multi-process mesh Bus backend | Phase D |
| MultiBus | Multi-backend composition layer | Phase D |
| Dreams online | Delta-speed consolidation loop becomes operational | — |
| Composer rewrite | HDC-driven retrieval and projection layers are prerequisites | — |
| Witness flows | Cross-deployment witness and replication | REF09 |
| Plugin registry | Published plugin registry with audit and semver guarantees | REF17 |

---

## Why Q5–Q6, not earlier

These items are intentionally deferred because:

1. **ChainBus, MeshBus**: distributed trust assumptions require the safety spine (Q4) and proven in-process Bus semantics (Q1) before they can be extended.
2. **Dreams**: the Delta-speed consolidation loop requires the full learning substrate (Q2) and stable storage/transport split.
3. **Composer rewrite**: meaningful only after HDC-driven retrieval (Q2) and projection layers (Q3) are stable.
4. **Plugin registry**: landing it before the SPI and audit model have settled would create premature API commitments.

---

## Exit criteria

- [ ] ChainBus and MeshBus have parity with the in-process Bus surface
- [ ] Replay behavior defined and tested for each backend
- [ ] Dreams operational in at least one self-hosting flow
- [ ] Plugin registry operational with at least one third-party plugin published

---

## Current status

Deferred. No active work planned.

---

## REF alignment

| REF | Scope |
|---|---|
| REF09 | Phase-2 Bus/Substrate backends |

---

## See also

- [`strategy/roadmap/milestone-q4-scale-safety-domains.md`](milestone-q4-scale-safety-domains.md) — prerequisite
- [`strategy/refactor-phases/04-phase-chain-mesh-buses.md`](../refactor-phases/04-phase-chain-mesh-buses.md) — Phase D mechanics
- [`strategy/roadmap/beyond-current-quarter.md`](beyond-current-quarter.md)
