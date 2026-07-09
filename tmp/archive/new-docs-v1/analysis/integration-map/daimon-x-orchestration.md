---
title: "Daimon × Orchestration"
section: analysis
subsection: integration-map
id: im-daimon-x-orchestration
source: 24-cross-section-integration-map.md (§6.2 M1, §3.2)
missing-integration: M1
tier: 1
tags: [daimon, orchestration, PAD, scheduling, affect, behavioral-state]
---

# Daimon × Orchestration

**Direction**: 09-Daimon → 01-Orchestration (PAD-modulated scheduling); 01-Orchestration → 09-Daimon is M11, see [orchestration-x-daimon.md](./orchestration-x-daimon.md)  
**Status**: **Missing (M1)** — Tier 1, ~60 LOC  
**Interface**: `roko-daimon::AffectState` → `orchestrate.rs::PlanRunner` tick scheduling

## What Flows

Daimon exposes a PAD vector (Pleasure-Arousal-Dominance) and a derived `BehavioralState` enum. The orchestrator currently treats all tasks with identical Theta cadence (reflection interval). M1 wires the behavioral state into the scheduling decision.

| Signal | From | To | Kind |
|---|---|---|---|
| `BehavioralState` enum | `roko-daimon::AffectState` | `orchestrate.rs` task scheduler | Config/runtime param |
| PAD vector | `roko-daimon` | Theta-interval formula | Config/runtime param |

## Wiring Recipe

```rust
// In orchestrate.rs, before each task dispatch:
let affect = daimon.current_state();
let theta_interval = match affect.behavioral_state {
    BehavioralState::Struggling => Duration::from_secs(30),  // More frequent reflection
    BehavioralState::Focused    => Duration::from_secs(120), // Less interruption
    BehavioralState::Coasting   => Duration::from_secs(90),  // Standard
    _                           => Duration::from_secs(75),  // Default Theta
};
```

Prerequisite: `AffectModel` trait in `roko-core` (see [AA-10 Improvement I6](../architectural-analysis/10-prioritized-improvements.md)).

## Invariants of the Interaction

1. The PAD vector always reflects the most recent affect state — the orchestrator reads it fresh each tick.
2. Behavioral state transitions must not cause scheduling starvation: even `Struggling` state must allow task execution, only shortening the *reflection* cadence.
3. A `Focused` state should suppress, not eliminate, Theta reflection.

## Failure Modes

| Failure | Consequence | Detection |
|---|---|---|
| Daimon unavailable at startup | Orchestrator falls back to default Theta interval (75s) | Log warning; continue |
| BehavioralState thrashes rapidly | Theta interval oscillates | Rate-limit state reads; minimum interval holds |
| Stale PAD state (Daimon not updating) | Orchestrator uses outdated scheduling signal | Timestamp check; warn if PAD older than 2× default interval |

## Observed Metrics

Not yet wired. Expected once implemented:
- `theta_interval_ms` per run — observable distribution of reflection cadences
- Correlation between `BehavioralState::Struggling` and subsequent task success rate

## Open Questions

1. Should a Struggling→Focused transition immediately extend the Theta interval, or only at the next scheduled Theta tick?
2. Does the orchestrator need to know the raw PAD vector (3 floats) or is `BehavioralState` sufficient?
3. Should M1 and M11 be implemented atomically as a bidirectional loop, or is one-way (M1 first) sufficient initially?

## Cross-References

- Reverse direction: [orchestration-x-daimon.md](./orchestration-x-daimon.md) — M11 (01→09)
- Composition side: [daimon-x-composition.md](./daimon-x-composition.md) — M2
- Architectural finding: [AA-06: Cross-Cut Isolation](../architectural-analysis/06-finding-crosscut-isolation.md) — Daimon injection gaps
- Improvement: [AA-10 I6](../architectural-analysis/10-prioritized-improvements.md) — AffectModel trait prerequisite
- Readiness audit: [RA-09: Daimon](../readiness-audit/subsystem-daimon.md)
