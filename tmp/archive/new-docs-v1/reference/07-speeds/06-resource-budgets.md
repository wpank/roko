# Resource Budgets per Speed Tier

> How compute, token, and cost budgets are allocated across Gamma, Theta, and Delta.

**Status**: Shipping
**Crate**: `roko-agent`
**Last reviewed**: 2026-04-19

---

## TL;DR

The budget controller allocates compute and cost budgets to each speed tier. The
default allocation assumes a typical workload: ~80% of ticks at Gamma, ~18% at Theta,
~2% overhead for Delta. Budgets are enforced per-tick and per-day. Exceeding the daily
budget triggers automatic throttling.

---

## Budget Dimensions

Each tick consumes three types of budget:

| Budget type | Unit | Where tracked |
|---|---|---|
| Token budget | tokens | Per-tick; enforced in COMPOSE |
| Cost budget | USD | Per-tick and per-day; enforced in ACT |
| Compute budget | CPU·ms | Per-tick; enforced by TickBudget |

---

## Per-Tick Token Budgets

| Tier | System prompt | Candidate context | Stimulus | Total max |
|---|---|---|---|---|
| Gamma | ~200 tokens | ~2 000 tokens | ~500 tokens | 4 096 tokens |
| Theta | ~500 tokens | ~12 000 tokens | ~1 500 tokens | 16 384 tokens |
| Delta | N/A (no model) | N/A | N/A | N/A |

---

## Per-Tick Cost Budgets (defaults)

| Tier | Max cost per tick | Typical actual cost |
|---|---|---|
| Gamma | $0.005 | < $0.001 |
| Theta | $0.25 | $0.01–$0.10 |
| Delta | $0 (no model call) | $0 |

The per-tick cost cap is enforced before the ACT stage:
```
if estimated_cost > budget.max_cost_per_tick {
    return ActError::CostExceeded
}
```

Estimated cost is computed from `composed.token_count × model.cost_per_token`.

---

## Daily Budget Allocation

For a deployment with a $10/day budget:

| Tier | Fraction | Daily allocation | Typical usage |
|---|---|---|---|
| Gamma | 80% | $8.00 | 8 000–80 000 ticks/day |
| Theta | 18% | $1.80 | 18–180 ticks/day |
| Delta | 2% | $0.20 | Compute overhead only |

These fractions are defaults. A research agent may invert the ratio (80% Theta), while
a monitoring agent may be 99% Gamma.

---

## Budget Enforcement

```toml
[budget]
daily_usd               = 10.00
gamma_fraction          = 0.80
theta_fraction          = 0.18
delta_fraction          = 0.02
gamma_max_tick_usd      = 0.005
theta_max_tick_usd      = 0.25
throttle_on_exceed      = true      # slow down rather than hard stop
throttle_period_secs    = 60        # minimum gap between ticks when throttled
```

When `throttle_on_exceed = true`, the agent slows its tick rate rather than stopping
entirely. This prevents agents from going silent unexpectedly. When
`throttle_on_exceed = false`, the agent stops all ticks until the next budget window.

---

## Compute Budget

Non-model stages (QUERY through REACT, minus ACT) are bounded by CPU time:

| Tier | Max CPU per tick |
|---|---|
| Gamma | 50 ms |
| Theta | 100 ms |
| Delta | 5 000 ms (background) |

These limits are enforced by `TickBudget.charge()`. A stage that runs over budget
is interrupted, and the tick proceeds with what was computed so far.

---

## See also

- [Gamma](01-gamma-reactive.md), [Theta](02-theta-reflective.md), [Delta](03-delta-consolidation.md)
- [Loop Performance](../06-loop/14-performance.md) — per-stage latency budgets
- [Operations / configuration](../../operations/configuration/README.md) — budget config reference
