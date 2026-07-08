# Loop Performance

> Per-stage latency budgets, P99 targets, tail-latency control, and cost accounting.

**Status**: Shipping
**Crate**: `roko-agent`
**Depends on**: [loop\_tick()](09-loop-tick-code.md),
[Three Cognitive Speeds](../07-speeds/README.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

The cognitive loop's non-ACT stages complete in < 100 ms combined. The dominant source
of tick latency is the ACT stage (model API call: 500 ms–30 s). Performance work
should focus on model selection, routing, and context size rather than loop overhead.
Per-stage budgets are enforced via `TickBudget`; violations produce `budget.exceeded`
Pulses.

---

## Stage Latency Budgets

All values are wall-clock time. "Target" is the median; "P99" is the 99th percentile
hard limit that triggers timeout if exceeded.

| Stage | Median target | P99 budget | Notes |
|---|---|---|---|
| QUERY | 3 ms | 20 ms | HDC similarity search; dominated by substrate latency |
| SCORE | 1 ms | 5 ms | CPU-bound; O(N) in candidates |
| ROUTE (static) | 0.1 ms | 0.5 ms | Hash lookup |
| ROUTE (Wilson CI) | 0.5 ms | 3 ms | Simple statistics |
| ROUTE (LinUCB) | 2 ms | 12 ms | Linear model; cache-friendly |
| COMPOSE | 2 ms | 12 ms | Token counting + string assembly |
| ACT (model, T0) | 800 ms | 10 000 ms | External; p50 varies by model |
| ACT (model, T1) | 3 000 ms | 30 000 ms | Larger model; more tokens |
| ACT (tool, local) | 50 ms | 5 000 ms | Depends on tool |
| VERIFY (full pipeline) | 8 ms | 25 ms | Classifier inference included |
| PERSIST (in-process) | 1 ms | 3 ms | — |
| PERSIST (sled/NVMe) | 3 ms | 8 ms | — |
| REACT | 2 ms | 18 ms | Pulse publishing + scheduling |
| **Total (T0, ex-ACT)** | **20 ms** | **99 ms** | All non-ACT stages |
| **Total (T0, inc-ACT)** | **820 ms** | **10 100 ms** | Dominated by model call |

---

## Tick Budget Enforcement

The `TickBudget` struct tracks cumulative stage time and raises `budget.exceeded` if
the per-tick total exceeds the configured limit.

```rust
// source: crates/roko-agent/src/budget.rs
pub struct TickBudget {
    pub total_ms: u64,       // hard limit (default: 60_000 ms)
    pub act_ms:   u64,       // budget for ACT specifically
    pub consumed: AtomicU64, // running total
}

impl TickBudget {
    pub fn charge(&self, stage: &str, millis: u64) -> Result<(), BudgetError> {
        let new_total = self.consumed.fetch_add(millis, Ordering::Relaxed) + millis;
        if new_total > self.total_ms {
            Err(BudgetError::Exceeded { stage: stage.to_string(), consumed: new_total })
        } else {
            Ok(())
        }
    }
}
```

When `BudgetError::Exceeded` is returned, `loop_tick()` aborts and publishes
`budget.exceeded`. The tick's Provenance Engram records the per-stage breakdown so
operators can identify which stage consumed the budget.

---

## Cost Accounting

Every tick records token costs in the Provenance Engram:

```rust
pub struct TickCost {
    pub prompt_tokens:     u32,
    pub completion_tokens: u32,
    pub model_cost_usd:    f64,
    pub compute_cost_usd:  f64,   // Roko's own compute
    pub total_cost_usd:    f64,
}
```

The `model_cost_usd` is computed from the actual `ActOutput.token_cost`. The
`compute_cost_usd` is estimated from stage wall times × configured compute rate.

Cumulative cost tracking per agent per day is available via the Substrate query:
```
SELECT SUM(tick_cost.total_cost_usd)
FROM engrams
WHERE kind = 'Provenance'
  AND agent_id = ?
  AND created_at > now() - 24h
```

---

## Tail-Latency Control

P99 latency is controlled by three mechanisms:

### 1. Stage timeouts (see [Failure Modes](13-failure-modes.md))

Every stage has an independent timeout. A slow VERIFY (e.g., a sluggish safety
classifier) does not block PERSIST indefinitely.

### 2. Candidate caps

`QuerySpec.max_results` (default 64 for T0/T1) prevents the SCORE stage from scaling
to O(N) on large substrates. The cap is tuned so SCORE always completes in < 5 ms.

### 3. Context token limits

The `ComposerContext.token_budget` prevents COMPOSE from assembling contexts that
cause ACT to time out. A model that receives a 120 000-token prompt will take much
longer than one that receives 4 096 tokens.

---

## Benchmarks (from `crates/roko-agent/benches/loop_bench.rs`)

```
test loop_tick_T0_in_memory      ... bench:   1_847_341 ns/iter (+/- 123_000)  # ~1.85 ms (ex-ACT)
test loop_tick_T1_in_memory      ... bench:   4_921_822 ns/iter (+/- 401_000)  # ~4.9 ms (ex-ACT)
test loop_tick_T0_sled_substrate ... bench:   5_912_000 ns/iter (+/- 890_000)  # ~5.9 ms (ex-ACT)
test loop_tick_T1_sled_substrate ... bench:  11_430_000 ns/iter (+/- 1_100_000) # ~11.4 ms (ex-ACT)
```

These benchmarks use a mock ACT stage (returns immediately). Real-world tick latency
is dominated by the ACT stage.

---

## Optimization Priorities

In order of expected impact:

1. **Model selection** — cheaper, faster models for T0 ticks reduce the dominant cost.
   The CascadeRouter's Wilson CI path learns which model gives acceptable quality for
   each stimulus type.
2. **Context size** — smaller contexts = faster model calls. Tune `token_budget` per
   speed tier.
3. **Substrate choice** — in-process substrate eliminates QUERY I/O; use for latency-
   critical deployments.
4. **Candidate cap** — lower the cap for T0 ticks if SCORE latency is a concern.
5. **Gate pipeline** — disable expensive gates (safety classifier) for low-risk
   deployment environments.

---

## See also

- [Failure Modes](13-failure-modes.md) — timeout behavior and recovery
- [loop\_tick() reference](09-loop-tick-code.md) — where TickBudget is threaded through
- [Three Cognitive Speeds](../07-speeds/README.md) — per-tier budgets
- [Operations / performance](../../operations/performance/README.md) — production tuning guide
