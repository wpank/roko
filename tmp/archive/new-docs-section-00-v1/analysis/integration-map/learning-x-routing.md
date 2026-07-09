---
title: "Learning × Routing"
section: analysis
subsection: integration-map
id: im-learning-x-routing
source: 24-cross-section-integration-map.md (§6.2 M6, §5.1-5.2)
missing-integration: M6
tier: 1
tags: [learning, routing, cost, budget, cascade-router, cost-guardrails]
---

# Learning × Routing

**Direction**: 05-Learning (internal — cost state → CascadeRouter)  
**Status**: **Partially Wired** — CascadeRouter health-based routing is wired (Loop 1); budget guardrails in CascadeRouter are **Missing** (M6 gap)  
**Interface**: Budget state from config/runtime → `roko-learn::CascadeRouter` stage-1 filtering

## What Flows

| Signal | From | To | Status |
|---|---|---|---|
| Provider health metrics | Learning runtime | CascadeRouter (stage-1) | **Wired** — Loop 1 |
| Confidence threshold | Config `routing.confidence_threshold` | CascadeRouter (stage-2) | **Wired** |
| Budget remaining (`budget.max_plan_usd`) | Config → runtime cost tracker | CascadeRouter tier filtering | **Missing** (M6) |
| Session cost accumulated | Cost tracker (in-memory) | CascadeRouter cheap-model bias | **Missing** (partial) |
| Latency SLA (`routing.latency_sla_ms`) | Config | CascadeRouter latency reward | **Missing** |

## The M6 Gap: Cost Guardrails Not Wired

**Problem**: The `budget.max_plan_usd` and `budget.max_session_usd` config parameters exist but are not read by the CascadeRouter. When budget is low, routing should bias toward cheaper models — this does not happen.

### Wiring Recipe

```rust
// In CascadeRouter::select_tier() or equivalent:
let budget_state = cost_tracker.current_state(); // new: inject CostTracker

let tier_filter = if budget_state.remaining_fraction() < 0.2 {
    // Under 20% budget remaining → only cheapest tier
    TierFilter::CheapOnly
} else if budget_state.remaining_fraction() < 0.5 {
    // Under 50% → prefer cheap, allow medium on high confidence
    TierFilter::PreferCheap
} else {
    TierFilter::All
};

// Apply tier filter before confidence-based selection
candidates.retain(|c| tier_filter.allows(c.tier));
```

Also needed: wire `routing.latency_sla_ms` as a bandit reward component (Loop 7 in feedback loops doc, ~35 LOC within Learning section).

Estimated LOC for M6: ~70 (source file 24, §6.2 M6).

## Invariants of the Interaction

1. Budget guardrails never prevent task execution — they only bias model selection, not block it.
2. When all models are filtered out by budget, fall back to cheapest available model (never `None`).
3. Cost tracking is session-scoped and plan-scoped; both limits apply.
4. Budget state is read-only by the Router; it does not modify the cost tracker.

## Failure Modes

| Failure | Consequence | Detection |
|---|---|---|
| Cost tracker not initialized | No budget filtering | Log warning; `TierFilter::All` fallback |
| Budget exhausted mid-plan | Expensive models still selected | Cost tracker must update after each agent call |
| Config missing `max_plan_usd` | No plan-level guardrail | Default to session-level only; warn |
| Budget tracking race (concurrent tasks) | Over-budget spending | Atomic cost accumulation |

## Observed Metrics

Expected after implementation:
- Model tier distribution per budget consumption decile
- Over-budget incident rate (should be 0 after M6)
- Cost per successful plan (before and after M6)

## Open Questions

1. Should cost guardrails be strict (never exceed budget) or soft (warn and bias, but don't block)?
2. Is session budget separate from plan budget in the `Cost` type, or does the same tracker serve both?
3. Should the CascadeRouter expose a `budget_remaining` signal back to the orchestrator for proactive replanning?

## Cross-References

- Sibling Tier 1: [learning-x-composition.md](./learning-x-composition.md) — M4
- Conductor influence: [conductor-x-routing.md](./conductor-x-routing.md) — M9 (system load also biases routing)
- Configuration analysis: [00-overview.md](./00-overview.md) §5 (Configuration Flow Map)
- Readiness audit: [RA-05: Learning](../readiness-audit/subsystem-learning.md)
