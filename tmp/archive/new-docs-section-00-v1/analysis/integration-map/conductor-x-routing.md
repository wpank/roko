---
title: "Conductor × Routing"
section: analysis
subsection: integration-map
id: im-conductor-x-routing
source: 24-cross-section-integration-map.md (§6.1 M9, §5.2)
missing-integration: M9
tier: 2
tags: [conductor, routing, system-load, load-based-routing, circuit-breaker, cascade-router]
---

# Conductor × Routing

**Direction**: 07-Conductor → 05-Learning (system-load signal → CascadeRouter)  
**Status**: **Missing (M9)** — Tier 2, ~45 LOC. `SystemLoadSnapshot` exists; CascadeRouter does not read it.  
**Interface**: `roko-conductor::SystemLoadSnapshot` → `roko-learn::CascadeRouter`

## What Flows

The Conductor continuously monitors system health (agent count, memory, CPU, circuit breaker state). Under high load, routing should prefer cheaper/faster models to prevent resource exhaustion. This signal is missing from the CascadeRouter.

| Signal | From | To | Status |
|---|---|---|---|
| `SystemLoadSnapshot` (agent count, memory, circuit state) | `roko-conductor` | `CascadeRouter` | **Missing** (M9) |
| `conductor.max_agents` config | Config | Routing bias | **Missing** |
| Circuit breaker open/closed state | `CircuitBreaker` | Model tier eligibility | **Missing** |

## Wiring Recipe

```rust
// In CascadeRouter selection logic:
let load = conductor.system_load_snapshot();  // new: inject conductor snapshot

// Under high load → prefer cheap models
let load_factor = if load.active_agents > load.max_agents * 0.8 {
    TierBias::CheapOnly
} else if load.memory_pressure > 0.9 {
    TierBias::PreferCheap
} else {
    TierBias::None
};

// Apply load bias alongside budget and confidence biases
let tier_candidates = apply_tier_bias(base_candidates, load_factor);
```

Estimated LOC: ~45 (source file 24, §6.1 M9).

See also: feedback loops doc Loop 2 (Conductor→Routing partial wiring; M9 completes it).

## Invariants of the Interaction

1. Load-based routing is a bias, not a block — the system never refuses to execute a task due to load.
2. The conductor snapshot is read at routing time, not persisted in the Router.
3. When the circuit breaker is open for a provider, that provider's models are excluded from routing candidates.
4. `max_agents` config from `conductor.max_agents` is the load threshold denominator.

## Failure Modes

| Failure | Consequence | Detection |
|---|---|---|
| Conductor unavailable | No load signal; routing uses default (no bias) | Log warn; `TierBias::None` fallback |
| Load snapshot stale | Routing uses outdated load data | Timestamp on snapshot; warn if >30s old |
| Circuit breaker open but not read | Requests routed to failed provider | Circuit state must be checked before routing |

## Observed Metrics

Expected after implementation:
- Model tier distribution correlated with system load
- Incidence of load-induced tier downgrade
- Success rate at each load tier vs baseline

## Open Questions

1. Should load-based routing also affect the Theta reflection interval (connection to M1)?
2. Is there a minimum load threshold below which the signal is ignored to prevent unnecessary tier downgrading?

## Cross-References

- Budget routing: [learning-x-routing.md](./learning-x-routing.md) — M6 (both affect CascadeRouter tier selection)
- Conductor-orchestration: [conductor-x-orchestration.md](./conductor-x-orchestration.md) — wired connection (sibling flow)
- Readiness audit: [RA-07: Conductor](../readiness-audit/subsystem-conductor.md), [RA-05: Learning](../readiness-audit/subsystem-learning.md)
