---
title: "Coordination × Orchestration"
section: analysis
subsection: integration-map
id: im-coordination-x-orchestration
source: 24-cross-section-integration-map.md (§6.1 M12, §3.3)
missing-integration: M12
tier: 4
tags: [coordination, orchestration, pheromones, stigmergy, agent-mesh, multi-agent]
---

# Coordination × Orchestration

**Direction**: 13-Coordination → 01-Orchestration (pheromone signals inform task scheduling)  
**Status**: **Missing (M12)** — Tier 4, ~150 LOC. Blocked on Agent Mesh transport (Readiness Audit G26).  
**Interface**: `Kind::Pheromone` Engrams from coordination subsystem → `roko-orchestrator::PlanRunner`

## What Flows

In a multi-agent setting, pheromones encode indirect coordination signals — "this path has been tried and failed," "this resource is currently occupied," "this subtask is progressing well." The orchestrator should factor these signals into task scheduling and resource allocation.

| Signal | From | To | Status |
|---|---|---|---|
| `Kind::Pheromone` (path success/failure markers) | `roko-core` (pheromone types) | `roko-orchestrator` task scheduler | **Missing** — no pheromone types in code yet (G19) |
| Agent Mesh occupancy signals | Agent Mesh transport | Orchestrator load-balancing | **Missing** — Agent Mesh not built (G26) |
| Morphogenetic state | `MorphogeneticState` | Plan decomposition hints | **Missing** — zero code |

## Reality Check

This integration is Tier 4 because it depends on:
1. Pheromone types existing in `roko-core` (G19 — a few hours of work)
2. Agent Mesh transport (G26 — Very High complexity, Phase 2+)

For single-agent Roko, pheromone signals from a previous session (stored in NeuroStore with `Decay::HalfLife`) provide a limited form of coordination across time, even without multi-agent mesh.

## Wiring Recipe (Single-Agent, Near-Term)

```rust
// In orchestrate.rs, during task prioritization:
let path_pheromones = substrate.query(Query::by_kind(Kind::Pheromone))
    .await?
    .filter(|p| !p.is_expired());

// Deprioritize tasks with high failure pheromone concentration
for task in &mut pending_tasks {
    let failure_signal = path_pheromones.iter()
        .filter(|p| p.relates_to(&task.id))
        .map(|p| p.signal_strength())
        .sum::<f32>();
    
    task.priority_bias -= failure_signal * 0.1;
}
```

Estimated LOC: ~150 (single-agent path; multi-agent path requires Agent Mesh).

## Invariants of the Interaction

1. Pheromone signals are advisory, not mandatory — the orchestrator can override them.
2. Pheromone Engrams are time-decaying (`Decay::HalfLife`) — stale signals have diminishing influence.
3. In single-agent mode, pheromone signals encode cross-session memory of path quality.
4. In multi-agent mode, pheromone signals encode real-time stigmergic coordination.

## Open Questions

1. Should single-agent pheromone emission be implemented immediately as a near-term Tier 1 item (G19 is described as "a few hours")?
2. What is the correct half-life for path pheromones? Too short → no cross-session memory; too long → stale signals mislead.

## Cross-References

- Dreams extension: [coordination-x-dreams.md](./coordination-x-dreams.md) — M19 (pheromone history as input to dream consolidation)
- Knowledge complement: [neuro-x-composition.md](./neuro-x-composition.md) — M5 (pheromones can supplement NeuroStore knowledge)
- Readiness audit: [RA-13: Coordination](../readiness-audit/subsystem-coordination.md), [RA-01: Orchestration](../readiness-audit/subsystem-orchestration.md)
