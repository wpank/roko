# 08-learning/09 -- Cost Normalization

> Blended cost per million tokens, token-type normalization, multi-level
> budget guardrails, CostsLog persistence, and the cost-to-routing feedback
> loop.

**Parent:** [08-LEARNING](../../08-LEARNING.md)

**Source:** `crates/roko-learn/src/costs_db.rs`,
`crates/roko-learn/src/costs_log.rs`

**Persistence:** `.roko/learn/costs.jsonl`

**Cross-references:** [cascade-router](cascade-router.md),
[task-metrics-and-baselines](task-metrics-and-baselines.md),
[provider-health-circuit-breaker](provider-health-circuit-breaker.md)

---

## 1. Purpose

Cost normalization provides a consistent framework for comparing model costs
across providers, pricing tiers, and token types. Raw cost data from providers
is heterogeneous: some charge per input token, some per output token, some per
request; cache hits reduce costs differently; reasoning tokens may have
distinct pricing. The cost normalization layer transforms this into a single
comparable metric -- blended cost per million tokens -- that the cascade router
and budget guardrails can use for routing decisions.

---

## 2. CostRecord Schema

```rust
pub struct CostRecord {
    pub timestamp: DateTime<Utc>,
    pub model: String,
    pub provider: String,
    pub role: String,
    pub plan_id: String,
    pub task_id: String,
    pub complexity_band: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
    pub duration_ms: u64,
    pub success: bool,
    pub session_id: String,
}
```

### 2.1 CostSummary

```rust
pub struct CostSummary {
    pub total_records: usize,
    pub total_cost_usd: f64,
    pub avg_cost_usd: f64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub avg_cost_per_success: f64,
    pub success_count: usize,
}
```

---

## 3. Blended Cost Formula

The blended cost per million tokens uses a 3:1 input-to-output weighting
ratio, following the Artificial Analysis methodology:

```
blended_cost_per_m = (3 * input_price_per_m + 1 * output_price_per_m) / 4
```

### 3.1 Why 3:1?

The ratio reflects the typical token mix in agent workloads: agents read more
than they write. In Roko's measured workloads, the median input-to-output
ratio is approximately 3:1 (for every output token, ~3 input tokens in
context, prior conversation, and tool results).

### 3.2 Token-Type Normalization

| Token Type | Typical Pricing | Normalization |
|------------|----------------|---------------|
| Fresh input tokens | Full input price | 1.0x |
| Cache read tokens | 10-90% discount | Weighted by actual cache price |
| Cache write tokens | Usually same as input | 1.0x |
| Reasoning/thinking tokens | Usually same as output | Counted as output |
| System prompt tokens | Full input price | 1.0x (but often cached) |

---

## 4. CostTable

```rust
pub struct ModelPricing {
    pub model: String,
    pub provider: String,
    pub input_price_per_m: f64,
    pub output_price_per_m: f64,
    pub cache_read_price_per_m: Option<f64>,
    pub blended_cost_per_m: f64,
}
```

Loaded from configuration and updated periodically as providers change pricing.

---

## 5. Budget Guardrails

```rust
pub struct BudgetGuardrail {
    pub per_task_limit: f64,
    pub per_session_limit: f64,
    pub per_day_limit: f64,
}

pub enum BudgetAction {
    Continue,
    Downgrade,   // triggered at 80% of limit
    Block,       // triggered at 95% of limit
    HardStop,    // triggered at 100% of limit
}
```

### 5.1 Escalation Thresholds

| Level | % of Limit | Action | Rationale |
|-------|------------|--------|-----------|
| Normal | < 80% | Continue | Full freedom |
| Warn | 80% | Downgrade | Route to cheaper model |
| Block | 95% | Block | Reject new requests |
| Hard stop | 100% | HardStop | Terminate session |

The downgrade at 80% is a soft intervention: the router automatically selects
a cheaper model rather than failing.

### 5.2 Multi-Level Enforcement

```
Incoming request
    |
    v
Check per-task budget
    | if >= 80% -> Downgrade
    | if >= 95% -> Block
    |
    v
Check per-session budget
    | if >= 80% -> Downgrade
    | if >= 95% -> Block
    |
    v
Check per-day budget
    | if >= 80% -> Downgrade
    | if >= 100% -> HardStop
    |
    v
Route to selected model (or cheaper alternative)
```

---

## 6. CostsLog: Append-Only Persistence

```rust
pub struct CostsLog {
    path: PathBuf,
    fsync: bool,
}
```

Key operations:

- `CostsLog::at(path)` -- construct a log at `path` with fsync enabled.
- `CostsLog::append(record)` -- append one `CostRecord` as a JSON line.
- `CostsLog::append_all(records)` -- batch append with a single open/close.
- `CostsLog::read_all(path)` -- read all records, tolerant of corrupt lines.

`CostsDb` is the in-memory companion used for real-time queries. `CostsLog`
provides durability: each completed call is appended, and the log is replayed
on startup to reconstruct the in-memory database.

---

## 7. Cost-to-Routing Feedback Loop

Cost data feeds back into routing through two paths:

1. **Direct cost penalty** -- the confidence stage subtracts a cost penalty
   from each model's score, biasing toward cheaper models when pass rates
   are similar.
2. **Budget guardrail enforcement** -- when limits approach, the guardrail
   forces cheaper models or blocks requests.

This creates cybernetic feedback loop 6 (Cost->Routing): higher costs trigger
routing changes that reduce costs, which relaxes budget pressure, which may
allow routing back to better (more expensive) models.
