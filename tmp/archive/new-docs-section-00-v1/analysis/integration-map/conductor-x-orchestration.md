---
title: "Conductor × Orchestration"
section: analysis
subsection: integration-map
id: im-conductor-x-orchestration
source: 24-cross-section-integration-map.md (§3.1, §4.1)
tags: [conductor, orchestration, circuit-breaker, health-monitoring, wired]
---

# Conductor × Orchestration

**Direction**: Bidirectional — Orchestration sends load signals to Conductor; Conductor returns circuit state  
**Status**: **Wired**  
**Interface**: `roko-orchestrator::PlanRunner` ↔ `roko-conductor::CircuitBreaker` + `DiagnosisEngine`

## What Flows

| Signal | From | To | Status |
|---|---|---|---|
| System load signal (active agents, plan state) | `roko-orchestrator` | `roko-conductor` | **Wired** |
| `Kind::Intervention` (circuit breaker decisions) | `roko-conductor` | `roko-orchestrator` | **Wired** |
| `DiagnosisEngine` error categorization | `roko-conductor` | `roko-orchestrator` retry logic | **Wired** |
| Per-plan failure history | `CircuitBreaker` | Plan state machine | **Wired** |

## Invariants of the Interaction

1. Circuit breaker state transitions are idempotent — multiple concurrent reads return consistent state.
2. `ConductorPolicy` is a `Policy` trait impl — it participates in the standard pipeline.
3. Conductor decisions are logged as `Kind::Intervention` Engrams for the learning subsystem.
4. The Conductor does not modify plan state directly — it emits signals that the orchestrator acts on.

## Enhancement Opportunities

- [conductor-x-routing.md](./conductor-x-routing.md) — M9: system load should also bias model routing

## Cross-References

- Layer issue: [../architectural-analysis/03-finding-layer-taxonomy.md](../architectural-analysis/03-finding-layer-taxonomy.md) — conductor→learn violation
- Readiness audit: [RA-07: Conductor](../readiness-audit/subsystem-conductor.md), [RA-01: Orchestration](../readiness-audit/subsystem-orchestration.md)
