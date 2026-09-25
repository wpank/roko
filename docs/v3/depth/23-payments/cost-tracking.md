# Cost Tracking

> Depth file for [23-PAYMENTS](../../23-PAYMENTS.md) -- CostsDb, CostsLog, and
> per-request LLM cost recording and querying.

---

## 1. Overview

Every LLM API request produces a `CostRecord` that tracks the model, provider,
tokens consumed, cost in USD, and execution metadata. The cost tracking system
has two complementary layers:

- **CostsDb** -- in-memory database for real-time queries during a run
- **CostsLog** -- append-only JSONL file for durable persistence across restarts

Both live in `crates/roko-learn/src/`.

---

## 2. CostRecord

```rust
// crates/roko-learn/src/costs_db.rs
pub struct CostRecord {
    pub timestamp: String,       // ISO-8601 UTC
    pub model: String,           // model slug
    pub provider: String,        // "anthropic", "openai", etc.
    pub role: String,            // agent role
    pub plan_id: String,         // plan identifier
    pub task_id: String,         // task identifier
    pub complexity_band: String, // "mechanical", "standard", "complex", "expert"
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_tokens: u64,      // prompt-cache hits (90% discount)
    pub cost_usd: f64,           // computed from tokens x price
    pub duration_ms: u64,        // wall-clock request time
    pub success: bool,
    pub session_id: String,      // run grouping
}
```

---

## 3. CostsDb (In-Memory)

`CostsDb` stores records in a `Vec<CostRecord>` behind a `parking_lot::RwLock`.
All read/write access is lock-protected.

### 3.1 Recording

```rust
pub fn record(&self, record: CostRecord);
pub fn record_batch(&self, records: Vec<CostRecord>);
```

### 3.2 Querying

Queries scan the vec with linear complexity -- acceptable for up to ~100k
records per run:

| Query | Returns |
|---|---|
| `summary()` | Aggregate across all records |
| `by_model(model)` | Filtered by model slug |
| `by_role(role)` | Filtered by agent role |
| `by_plan(plan_id)` | Filtered by plan |
| `by_complexity(band)` | Filtered by complexity band |
| `by_time_range(start, end)` | Filtered by timestamp range |
| `by_session(session_id)` | Filtered by run session |

### 3.3 CostSummary

```rust
pub struct CostSummary {
    pub total_cost_usd: f64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cached_tokens: u64,
    pub request_count: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub avg_duration_ms: f64,
    pub by_model: HashMap<String, f64>,    // model -> cost
    pub by_provider: HashMap<String, f64>, // provider -> cost
}
```

---

## 4. CostsLog (JSONL Persistence)

`CostsLog` provides durable, file-backed storage as an append-only JSONL file:

```rust
// crates/roko-learn/src/costs_log.rs
pub struct CostsLog {
    path: PathBuf,
    fsync: bool,   // default: true
}
```

### 4.1 Operations

| Method | Description |
|---|---|
| `at(path)` | Construct a log at a path |
| `open_creating(path)` | Create parent directories and open |
| `append(record)` | Write one CostRecord as one JSON line |
| `append_all(records)` | Batch write (amortizes syscall overhead) |
| `load_all()` | Read all records from disk |
| `load_since(timestamp)` | Read records after a timestamp |
| `without_fsync()` | Disable fsync for testing |

### 4.2 File Location

The costs log is written to `.roko/learn/costs.jsonl` within the workspace.

### 4.3 High-Throughput Path

For concurrent agents, the recommended pattern is collecting records into a
`Vec` and calling `append_all` in a periodic flush to amortize file I/O.

---

## 5. PaymentCostRecord

Paid feed requests and metered sessions produce a separate record type:

```rust
pub struct PaymentCostRecord {
    pub timestamp: String,
    pub feed_id: String,
    pub protocol: String,    // "x402" or "mpp"
    pub amount_korai: f64,
    pub payer: String,
    pub payee: String,
    pub session_id: String,
    pub settled: bool,
}
```

Payment records track x402 micropayments and MPP (Machine Payment Protocol)
session draws separately from inference costs.

---

## 6. CLI Inspection

```bash
roko show costs          # Summary of costs for current workspace
roko show costs --plan   # Per-plan cost breakdown
roko show costs --model  # Per-model cost breakdown
roko learn efficiency    # Cost efficiency metrics
```

---

## 7. Integration Points

| Consumer | How It Uses Costs |
|---|---|
| **Runner event loop** | Records cost after each provider call |
| **Graph cost state** | Paid-failure-aware enforcement with atomic reservations |
| **Budget enforcement** | Compares cumulative cost against ceilings |
| **CascadeRouter** | Uses cost data to optimize model selection |
| **Telemetry Lens** | Publishes cost events via E33 observation boundary |
| **Dashboard** | Shows real-time cost in TUI and HTTP surfaces |
| **ACP** | Persisted/enforced USD budgets per agent session |

---

## 8. Source Locations

| Component | Path |
|---|---|
| CostRecord + CostsDb | `crates/roko-learn/src/costs_db.rs` |
| CostsLog (JSONL) | `crates/roko-learn/src/costs_log.rs` |
| Budget config | `crates/roko-core/src/config/budget.rs` |
| Graph cost state | `crates/roko-graph/src/` |
| Efficiency events | `crates/roko-learn/src/` |

---

*New file for v3. Content derived from actual `roko-learn` implementation.
Cost tracking is fully wired into the runner and graph execution paths.*
