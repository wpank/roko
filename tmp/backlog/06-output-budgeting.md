# Output Budgeting: EMA-Based Per-Model max_tokens Cap

**Status:** Backlog
**Priority:** P2 (efficiency)
**Size:** S (0 days — already implemented)
**Origin:** `tmp/architecture-archive/07-gateway.md`, Stage 4 "Output budget"

---

## Problem Statement

Unconstrained `max_tokens` allows individual model calls to consume far more
output tokens than a task actually needs. For example, a task that normally
produces 400-token responses could be invoked with no `max_tokens` limit and,
under adversarial or runaway conditions, fill the entire context window. This
wastes budget and slows throughput.

The fix is to learn what each model actually produces across many calls, derive
a statistical upper bound, and enforce it as a soft cap — without touching calls
where the caller has set an explicit, reasonable limit.

---

## Proposed Solution

Track a per-model exponential moving average (EMA) of observed output token
counts. Once 20 observations accumulate, compute a p95 estimate as:

```
p95 = ema + 2 * sqrt(ema_sq - ema^2)
cap = p95 * 1.5, floor at 1024 tokens
```

Apply the cap to incoming requests only in two cases:

1. No `max_tokens` is set — insert the cap.
2. The caller's `max_tokens` exceeds `2 * cap` — reduce to the cap.

Do not reduce a caller's `max_tokens` that is within or below the cap.

EMA alpha: `0.05` (5% weight to each new observation). This resists outliers
while adapting over time.

---

## Implementation Location

**Already fully implemented** in `crates/roko-gateway/src/output_budget.rs`.

Key types:

```rust
pub struct ModelOutputStats {
    pub ema: f64,
    pub ema_sq: f64,
    pub max_seen: u64,
    pub count: u64,
}

pub struct OutputBudgeter { ... }

impl OutputBudgeter {
    pub fn record_output(&self, model: &str, output_tokens: u64);
    pub fn apply_budget(&self, model: &str, current_max_tokens: Option<u32>) -> Option<u32>;
    pub fn stats(&self) -> OutputBudgetStats;
}
```

The `OutputBudgeter` is wired into the nine-stage `InferenceGateway` pipeline
in `crates/roko-gateway/src/gateway.rs` (Stage 4). The pipeline is exposed
through `POST /api/gateway/inference` in `roko-serve`, and pipeline stats
including `output_budgets_applied` and `output_tokens_bounded` are returned
by `GET /api/gateway/stats` via the `pipeline` field.

---

## Acceptance Criteria

All acceptance criteria are already met:

- [x] `ModelOutputStats::cap()` returns `None` for fewer than 20 observations.
- [x] After 20+ observations, `cap()` returns `p95 * 1.5` with a floor of 1024.
- [x] `apply_budget` inserts a cap when `max_tokens` is absent.
- [x] `apply_budget` reduces a cap when `max_tokens > 2 * cap`.
- [x] `apply_budget` is a no-op when `max_tokens` is within or below the cap.
- [x] Stats counters `output_budgets_applied` and `output_tokens_bounded` are
  tracked atomically.
- [x] Two unit tests in `output_budget.rs` cover the sample-threshold and the
  cap-application boundary conditions.

---

## Current State

**This feature is complete.** No implementation work is needed.

The only open question is whether the main inference path used by CLI agents
(`model_call_service.rs` in `roko-agent`) should also apply output budgeting.
Currently the `ModelCallService` does not consult `OutputBudgeter` — it calls
providers directly without the nine-stage pipeline. If CLI-dispatched agents
should benefit from output budgeting, a separate task is needed to wire
`OutputBudgeter` into `ModelCallService` or route CLI agent calls through the
gateway pipeline.

---

## References

- `crates/roko-gateway/src/output_budget.rs` — implementation and unit tests
- `crates/roko-gateway/src/gateway.rs` — Stage 4 pipeline wiring
- `crates/roko-serve/src/routes/gateway.rs` — HTTP surface (`pipeline` stats field)
- `tmp/architecture-archive/07-gateway.md` — original design (Section 7)
- `.roko/GAPS.md` — E26 entry: "Inference gateway 12/12 ... output/thinking controls"
