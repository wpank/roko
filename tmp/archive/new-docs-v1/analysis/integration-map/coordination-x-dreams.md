---
title: "Coordination × Dreams"
section: analysis
subsection: integration-map
id: im-coordination-x-dreams
source: 24-cross-section-integration-map.md (§6.1 M19)
missing-integration: M19
tier: 4
tags: [coordination, dreams, pheromones, collective-memory, consolidation]
---

# Coordination × Dreams

**Direction**: 13-Coordination → 10-Dreams (pheromone history as input to dream consolidation)  
**Status**: **Missing (M19)** — Tier 4, ~120 LOC. Depends on M7 (Dreams→Neuro) and M12 (Coordination→Orchestration).  
**Interface**: `Kind::Pheromone` Engram history → `DreamRunner` consolidation input

## What Flows

In multi-agent systems, pheromone trails encode collective knowledge about which strategies worked and which failed across the whole agent population. Dreams should consolidate this collective memory alongside individual episode history, elevating durable patterns into NeuroStore.

| Signal | From | To | Status |
|---|---|---|---|
| Historical `Kind::Pheromone` Engrams | `Substrate` (coordination events) | `DreamRunner` as consolidation input | **Missing** (M19) |
| Collective success patterns (multi-agent) | Pheromone trail analysis | `NeuroStore` collective insights | **Missing** |

## Dependencies

M19 requires:
- M7 (Dreams→Neuro) for the consolidation infrastructure
- Pheromone types in `roko-core` (G19)
- Agent Mesh transport for multi-agent pheromone trails (G26)

Single-agent pheromones (cross-session path markers) would enable a reduced version of M19 without the Agent Mesh.

## Wiring Recipe (Sketch)

```rust
// In DreamProjection::run_nrem_cycle():
// Include pheromone history alongside episode history
let pheromone_history = substrate.query(Query::by_kind(Kind::Pheromone))
    .await?
    .filter(|p| p.created_at > self.last_consolidation);

let collective_patterns = analyze_pheromone_patterns(&pheromone_history);
for pattern in collective_patterns {
    neuro_store.put(KnowledgeEntry::from_collective(pattern)).await?;
}
```

Estimated LOC: ~120.

## Open Questions

1. Should individual episode history and collective pheromone history be consolidated in separate passes or together?
2. How do collective insights differ from individual insights in NeuroStore? Should they have a `source: Collective` provenance field?

## Cross-References

- Prerequisite: [dreams-x-neuro.md](./dreams-x-neuro.md) — M7
- Prerequisite: [coordination-x-orchestration.md](./coordination-x-orchestration.md) — M12
- Readiness audit: [RA-13: Coordination](../readiness-audit/subsystem-coordination.md), [RA-10: Dreams](../readiness-audit/subsystem-dreams.md)
