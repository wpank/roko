# 20-gateway/03 -- Cost Accounting

> Actual-versus-naive cost computation, batch discount, three-axis attribution,
> and budget deduction mechanics.

**Parent:** [20-GATEWAY](../../20-GATEWAY.md), section 11

**Source:** `crates/roko-gateway/src/cost_track.rs`

---

## 1. Design Rationale

Every inference request has a real cost and a counterfactual cost. The real cost
reflects what was actually charged (with caching, batching, and token-type
differentiation). The naive cost reflects what would have been charged with no
optimization at all. The difference is the mechanical savings produced by the gateway.

Tracking both numbers enables:
- **Dashboard reporting:** show operators how much the gateway is saving
- **Budget enforcement:** deduct actual cost from agent handles
- **Learning feedback:** the cascade router uses cost/latency to adjust routing weights
- **Attribution:** per-agent, per-session, and per-model breakdowns for accounting

---

## 2. Model Pricing

The `CostTable` from `roko-learn` provides per-model pricing:

```rust
pub struct ModelPricing {
    pub input_per_m: f64,         // USD per 1M input tokens
    pub output_per_m: f64,        // USD per 1M output tokens
    pub cache_read_per_m: f64,    // USD per 1M cached input tokens
    pub cache_write_per_m: f64,   // USD per 1M cache creation tokens
    pub tokenizer_ratio: f64,     // cross-provider normalization factor
}
```

The cost table uses substring matching for model families (e.g., `"claude-sonnet"`
matches `"claude-sonnet-4-20250514"`). A model the table does not price has an
unknown cost: `compute_cost` returns an all-zero result and logs the model once,
as roko-learn's `CostTable::calculate` does, rather than pricing it at another
model's rates (bug-39d15f).

---

## 3. Cost Formula

### 3.1 Cost Breakdown

Per request, the gateway computes six cost components:

```
fresh_input   = (input_tokens - cache_read_tokens) * input_per_m / 1,000,000
cached_input  = cache_read_tokens * cache_read_per_m / 1,000,000
cache_write   = cache_creation_tokens * cache_write_per_m / 1,000,000
regular_out   = (output_tokens - reasoning_tokens) * output_per_m / 1,000,000
reasoning     = reasoning_tokens * output_per_m / 1,000,000
thinking      = thinking_tokens * output_per_m / 1,000,000
```

`cache_write_per_m` is the table's rate for populating prefix caches: Anthropic's
5-minute write is 1.25x the base input rate, and a provider with no write
surcharge bills a write as input (bug-0c0747).

**Note:** The canonical `ModelPricing` does not yet have a separate `reasoning_per_m`
rate. Both reasoning and thinking tokens use the `output_per_m` rate. When providers
differentiate pricing, the formula will use the dedicated rate.

### 3.2 Actual Cost

```rust
let undiscounted = breakdown.total();  // sum of all 6 components
let actual_cost = if is_batch {
    undiscounted * 0.5  // 50% batch discount
} else {
    undiscounted
};
```

### 3.3 Naive Cost

The cost that would have been charged with no caching at all:

```
naive_cost = total_input_tokens * input_per_m / 1,000,000
           + total_output_tokens * output_per_m / 1,000,000
```

### 3.4 Savings

```
savings = naive_cost - actual_cost
```

This can be positive (optimization helped), zero (no optimization applied), or
negative (cache creation surcharge exceeded the naive cost -- rare, only for very
short requests with heavy cache writes).

---

## 4. Batch Discount

Requests submitted through the batch API receive a **50% reduction** on actual cost.
The batch flag is set automatically when a request enters the `BatchQueue::submit`
path (`request.metadata.is_batch = true`).

The discount applies to the undiscounted total, not to individual components. This
matches the provider batch pricing model (e.g., Anthropic's `/v1/messages/batches`
endpoint).

---

## 5. Attribution

Every `CostRecord` includes full attribution:

```rust
pub struct CostRecord {
    pub agent_id: String,
    pub session_id: String,
    pub model: String,
    pub cost: CostResult,
}
```

The `CostTracker` maintains three aggregate indexes:

| Index | Key | Used By |
|-------|-----|---------|
| `by_agent` | `agent_id` | Per-agent cost dashboards, budget limits |
| `by_session` | `session_id` | Per-session cost reporting |
| `by_model` | `model` | Model cost comparison, routing feedback |

Each aggregate tracks:

```rust
pub struct CostAggregate {
    pub requests: u64,
    pub actual_cost: f64,
    pub naive_cost: f64,
    pub savings: f64,
}
```

---

## 6. Budget Deduction

After cost computation, the gateway deducts the actual cost from the agent handle's
budget counter. The deduction uses microdollar precision:

```rust
fn usd_to_microdollars(usd: f64) -> u64 {
    if !usd.is_finite() || usd <= 0.0 { 0 }
    else { (usd * 1_000_000.0).ceil().min(u64::MAX as f64) as u64 }
}

fn deduct_budget(budget: &AtomicU64, cost_microdollars: u64) {
    let _ = budget.fetch_update(Ordering::AcqRel, Ordering::Relaxed, |remaining| {
        Some(remaining.saturating_sub(cost_microdollars))
    });
}
```

Key properties:
- **Ceil rounding** ensures the agent is never undercharged
- **Saturating subtraction** prevents underflow (budget cannot go negative)
- **AcqRel ordering** on `fetch_update` ensures visibility across concurrent requests
- **AtomicU64** is shared across all clones of the same `InferenceHandle`

### Preflight Budget Check

Before provider dispatch, the gateway estimates the request cost using a rough
character-to-token approximation (chars / 4) and the configured or default `max_tokens`.
If the estimated cost exceeds the remaining budget, the request is rejected with
`GatewayError::BudgetExceeded` before any provider call occurs.

---

## 7. Gateway Reward Signal

After cost computation, the gateway emits a multi-objective reward to the cascade
router for its contextual bandit:

```rust
fn compute_gateway_reward(cost_usd: f64, latency_ms: u64) -> f64 {
    let cost_penalty = (cost_usd * 100.0).min(0.5);
    let latency_penalty = ((latency_ms as f64) / 60_000.0).min(0.3);
    (1.0 - cost_penalty - latency_penalty).max(0.1)
}
```

Properties:
- Base reward: **1.0** (every successful completion is positive)
- Cost penalty: up to **0.5** (saturates at $0.005 per request)
- Latency penalty: up to **0.3** (saturates at 60 seconds)
- Minimum reward: **0.1** (no completion is ever zero-valued)
- Cheaper and faster providers accumulate higher cumulative rewards

This signal feeds `CascadeRouter::record_observation`, which updates the UCB1 bandit's
per-model reward estimates for future routing decisions.

---

## 8. Durable Event Publication

Cost records are published as `GatewayEvent` entries through the optional
`GatewayEventWriter`. Each event captures the request ID, caller identity
(agent:session), model, provider, token counts, cost, latency, cache-hit flag,
success flag, and timestamp. Events are written on both success and failure paths.
