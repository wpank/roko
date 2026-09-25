# Funding and Budgets

> **v3 depth file** -- `/docs/v3/depth/36-lifecycle/funding-and-budgets.md`
> Canonical source: v1 `docs/v1/17-lifecycle/04-funding-and-budgets.md`
> Status: **Current** (BudgetConfig in `roko-agent/src/lifecycle.rs`;
> persisted/enforced USD budgets via ACP; cost tracking in
> `.roko/learn/efficiency.jsonl`; paid-failure-aware cost enforcement in
> `roko-graph`)

---

## 1. Resource Types

Every agent consumes resources. The budget system provides multi-level
guardrails that prevent runaway costs while preserving agent autonomy. Budget
exhaustion triggers graceful degradation -- not agent termination.

| Resource | Unit | Tracked by | Typical cost |
|----------|------|-----------|-------------|
| Inference tokens | Tokens per LLM call | `roko-agent` dispatcher | $0.25-$15/M input tokens |
| Compute time | Wall-clock seconds | Managed infrastructure | $0.025-$0.20/hr |
| Tool invocations | Per-call usage | `roko-std` dispatcher | Free (local); x402-gated (remote) |
| On-chain gas | Gas units * gas price | `roko-chain` wallet | $0.001-$5.00/tx |

---

## 2. Budget Configuration

```toml
[budget]
max_daily_inference_usd = 10.0    # Hard daily cap
# max_total_usd = 1000.0          # Optional lifetime cap
max_tokens_per_turn = 8192        # Per-turn token limit

warning_at = 0.7                  # Warn at 70% of daily budget
critical_at = 0.9                 # Critical alert at 90%

degradation = "cascade"           # "cascade" | "pause" | "notify-only"
```

```rust
// crates/roko-agent/src/lifecycle.rs

pub struct BudgetConfig {
    pub max_daily_inference_usd: f64,
    pub max_total_usd: Option<f64>,
    pub max_tokens_per_turn: u64,
    pub warning_threshold: f64,     // Default: 0.7
    pub critical_threshold: f64,    // Default: 0.9
    pub degradation_mode: DegradationMode,
}
```

---

## 3. Cost Tracking

Cost tracking happens at three levels:

### Per-Turn Tracking

Every LLM call records a `TurnCostRecord` to `.roko/learn/efficiency.jsonl`:

- Turn ID, model used, input/output/cache-read tokens.
- Estimated cost in USD.
- Cognitive tier (Gamma/Theta/Delta).
- Whether the turn was suppressed by T0 probes (zero LLM cost).

### Per-Day Aggregation

Daily summaries computed from per-turn records:

- Total inference, compute, and gas costs.
- Total turns and T0-suppressed turns.
- T0 suppression rate (fraction avoiding LLM calls).
- Cost per turn (mean).
- Model distribution (fraction of calls per model).

### Lifetime Tracking

Cumulative cost across the agent's entire lifetime:

- Total all-in cost since creation.
- Days active, average daily cost.
- Projected monthly cost at current rate.

---

## 4. Multi-Level Guardrails

Budget enforcement operates at four levels:

### Level 1: Per-Turn Token Limit

`max_tokens_per_turn` (default: 8192) prevents any single LLM call from
consuming excessive tokens. Enforced at the dispatcher level before the
call is made. If a turn would exceed the limit, the context is compressed
to fit within budget.

### Level 2: Per-Hour Inference Rate

Sliding-window rate limiter prevents burst spending. Default: no more than
20% of daily budget consumed in any single hour. Prevents a misbehaving loop
from exhausting the entire daily budget in minutes.

### Level 3: Daily Budget Ceiling

`max_daily_inference_usd` (default: $10.00) is a hard daily cap. When
reached, the degradation cascade activates.

### Level 4: Lifetime Budget Cap

Optional `max_total_usd` provides an absolute spending limit. When reached,
the agent enters permanent pause until the operator increases the cap or
deletes the agent.

---

## 5. Graceful Degradation Cascade

When budget constraints are hit, the agent degrades gracefully through
five stages:

### Stage 1: Model Downgrade (at 70% daily budget)

The cascade router switches to cheaper models:
- Delta calls: claude-opus-4-6 -> claude-sonnet-4-6.
- Theta calls: claude-sonnet-4-6 -> claude-haiku-4-5.
- Gamma calls: remain on cheapest model.
- **Cost reduction**: ~60-80% per inference call.

### Stage 2: T0 Probe Emphasis (at 80% daily budget)

T0 probes (zero-LLM probes) activated at maximum sensitivity. T0 probes
handle ~80% of routine decisions without any LLM call:
- Cache probe: answers from cached knowledge entries.
- Pattern probe: matches against known patterns.
- Threshold probe: evaluates numeric conditions.
- Template probe: fills templated responses.
- **Cost reduction**: ~80% of turns suppressed.

### Stage 3: Reduced Tick Frequency (at 90% daily budget)

Tick intervals increased 4x:
- Gamma: 15s -> 60s.
- Theta: 75s -> 300s.
- Delta: 6h -> 24h.
- **Cost reduction**: ~75% fewer inference calls per hour.

### Stage 4: Monitoring Only (at 95% daily budget)

- No actions taken (no tool invocations, no coordination writes).
- Continues observing and logging.
- Knowledge store continues receiving new Signals (from monitoring).
- Affect engine transitions to Resting state.
- **Cost reduction**: ~95%.

### Stage 5: Budget Pause (at 100% daily budget)

- Cognitive loop pauses entirely.
- Agent process remains alive but idle.
- Health server continues responding.
- No inference calls.
- Resumes automatically when the daily budget window resets (midnight UTC).

The operator is notified at each stage transition. At no stage is the agent
deleted -- budget exhaustion is a resource constraint, not a lifecycle event.

---

## 6. Cost Efficiency Metrics

Efficiency metrics captured per-turn in `.roko/learn/efficiency.jsonl` drive
model routing improvements via the CascadeRouter:

| Metric | What it measures | Target |
|--------|-----------------|--------|
| T0 suppression rate | Turns handled without LLM | >80% |
| Cost per gate pass | Average cost of a verified turn | <$0.01 |
| Model distribution | Calls per model tier | >60% Haiku, <5% Opus |
| Token efficiency | Useful output / total tokens | >0.4 |
| Cache hit rate | Tokens served from cache | >0.3 |

Over time, the agent learns which decisions require expensive models and
which can be handled cheaply -- a form of metabolic optimization driven by
budget constraints and learning, not mortality pressure.

---

## 7. Funding Sources (Chain Domain)

Chain-domain agents have additional funding mechanisms:

1. **Direct USDC transfer** -- operator sends funds to the agent's wallet.
2. **x402 micropayments** -- EIP-3009 signed transfers for pay-per-use
   compute and inference. Each payment extends the compute budget.
3. **Metabolic self-funding** -- agents that earn revenue (trading, LP
   management, services) can fund their own operation. Formula:
   `F = (daily_cost * duration) * safety_margin` (default margin: 1.5x).
4. **Permissionless extensions** -- anyone can extend any agent's budget by
   sending KORAI payment. Payer type tracked for attribution.

---

## 8. Implementation Sources

| Surface | File | What |
|---------|------|------|
| `BudgetConfig` | `crates/roko-agent/src/lifecycle.rs` | Budget limits struct |
| Cost tracking | `crates/roko-cli/src/runner/` | Per-turn efficiency recording |
| Cascade router | `crates/roko-learn/src/` | Model routing based on cost/quality |
| ACP budgets | `crates/roko-acp/src/` | Persisted/enforced USD budgets |
| Graph cost | `crates/roko-graph/src/` | Paid-failure-aware cost enforcement |

---

## Cross-References

- [configuration-operator-model.md](configuration-operator-model.md) -- Budget in config
- [knowledge-demurrage.md](knowledge-demurrage.md) -- Token-level knowledge decay
- [academic-foundations.md](academic-foundations.md) -- Jonas (1966) metabolic economics
