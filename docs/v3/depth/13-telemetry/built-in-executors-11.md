# Depth 13-03: Built-In Lens Executors

> All 11 built-in Lens executors detailed: their event families, payload
> types, computed metrics, and chaining relationships.

**Parent**: [13-TELEMETRY](../../13-TELEMETRY.md) -- Section 3

---

## 1. Executor Catalog

The 11 built-in Lens executors divide into two tiers:

- **Primary Lenses** (9): observe raw lifecycle events from the
  execution engine
- **Derived Lenses** (2): observe the output of primary Lenses and
  compute second-order metrics

Every executor name follows the pattern `roko:{name}-lens@^1`.
The `observes_for_block` function in `lens_registry.rs` maps each
block name to its default event-family filters.

---

## 2. Primary Lenses

### 2.1 Cost Lens

**Block**: `roko:cost-lens@^1`
**Observes**: `CellLifecycle`, `GraphLifecycle`, `AgentLifecycle`

Accumulates USD spend across Cell completions and Graph completions,
broken down by model. Projects the `CostReportPayload`:

```rust
pub struct CostReportPayload {
    pub target: String,
    pub interval_ms: u64,
    pub total_usd: f64,
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub model_breakdown: BTreeMap<String, f64>,
    pub cumulative_usd: f64,
    pub budget_remaining: Option<f64>,
    pub vitality: Option<f64>,
}
```

The metric-oriented `CostLens` in `obs/lens.rs` projects the
`roko_llm_cost_usd_total` counter family, keyed by `provider` and
`model`.

### 2.2 Latency Lens

**Block**: `roko:latency-lens@^1`
**Observes**: `CellLifecycle`, `GraphLifecycle`

Collects execution durations from `CellCompleted` and
`GraphNodeCompleted` events. Projects the `LatencyPayload`:

```rust
pub struct LatencyPayload {
    pub target: String,
    pub interval_ms: u64,
    pub count: u64,
    pub p50_ms: u64,
    pub p95_ms: u64,
    pub p99_ms: u64,
    pub mean_ms: u64,
}
```

The metric-oriented `LatencyLens` in `obs/lens.rs` projects the
`roko_llm_ttft_seconds` and `roko_llm_request_duration_seconds`
histogram families.

### 2.3 Quality Lens

**Block**: `roko:quality-lens@^1`
**Observes**: `VerifyLifecycle`, `SignalLifecycle`

Aggregates verification verdicts from `VerifyPreResult` and
`VerifyPostResult` events. Tracks pass/fail counts per rung. Projects
the `QualityPayload`:

```rust
pub struct QualityPayload {
    pub target: String,
    pub interval_ms: u64,
    pub total_verifications: u64,
    pub pre_verify_vetoes: u64,
    pub post_verify_passed: u64,
    pub post_verify_failed: u64,
    pub pass_rate: f64,
    pub avg_reward: f64,
    pub hard_criteria_failures: u64,
    pub rung_breakdown: BTreeMap<String, PassFailCounts>,
}
```

### 2.4 Efficiency Lens

**Block**: `roko:efficiency-lens@^1`
**Observes**: `CellLifecycle`, `AgentLifecycle`

Computes per-agent task throughput, cost efficiency, and cache hit
rates. Projects the `EfficiencyPayload`:

```rust
pub struct EfficiencyPayload {
    pub agent: String,
    pub interval_ms: u64,
    pub tasks_completed: u64,
    pub tokens_per_task: f64,
    pub usd_per_task: f64,
    pub quality_per_usd: f64,
    pub t0_hit_rate: f64,
    pub t1_hit_rate: f64,
    pub t2_hit_rate: f64,
    pub avg_prediction_error: f64,
    pub vitality: f64,
    pub vitality_phase: String,
}
```

### 2.5 Error Lens

**Block**: `roko:error-lens@^1`
**Observes**: `CellLifecycle`, `GraphLifecycle`, `ExtensionLifecycle`

Classifies errors from `CellFailed`, `GraphFailed`, and
`ExtensionHookFailed` events into six categories and tracks retry
outcomes. Projects the `ErrorPayload`:

```rust
pub struct ErrorPayload {
    pub target: String,
    pub interval_ms: u64,
    pub total_errors: u64,
    pub by_category: BTreeMap<String, u64>,
    pub by_block: BTreeMap<String, u64>,
    pub retry_count: u64,
    pub retry_success_rate: f64,
    pub error_rate: f64,
}
```

The six error categories:

| Category | When classified |
|----------|----------------|
| `Timeout` | Execution exceeded deadline |
| `CapabilityDenied` | Missing Cell/Graph/Space capability |
| `External` | Provider or network failure |
| `LogicError` | Internal assertion or invariant violation |
| `InputInvalid` | Malformed task input |
| `Cancelled` | User or system cancellation |

### 2.6 Drift Lens

**Block**: `roko:drift-lens@^1`
**Observes**: `MemoryLifecycle`, `SignalLifecycle`

Monitors knowledge store quality changes. Tracks tier distribution,
balance decay, promotion and demotion rates, and anti-knowledge
accumulation. Projects the `DriftPayload`:

```rust
pub struct DriftPayload {
    pub memory: String,
    pub interval_ms: u64,
    pub total_entries: u64,
    pub tier_distribution: BTreeMap<String, u64>,
    pub avg_balance: f64,
    pub balance_delta: f64,
    pub promotion_rate: f64,
    pub demotion_rate: f64,
    pub cold_entries: u64,
    pub anti_knowledge_count: u64,
    pub heuristic_calibration_avg: f64,
}
```

### 2.7 Budget Lens

**Block**: `roko:budget-lens@^1`
**Observes**: `AgentLifecycle`, `CellLifecycle`

Projects budget consumption, burn rate, and projected exhaustion.
Assigns alert levels. Projects the `BudgetAlertPayload`:

```rust
pub struct BudgetAlertPayload {
    pub target: String,
    pub budget_total: f64,
    pub budget_spent: f64,
    pub budget_remaining: f64,
    pub vitality: f64,
    pub vitality_phase: String,
    pub projected_exhaustion_ms: Option<i64>,
    pub burn_rate: f64,
    pub level: AlertLevel,  // Info | Warning | Critical
}
```

### 2.8 Usage Lens

**Block**: `roko:usage-lens@^1`
**Observes**: `CellLifecycle`, `GraphLifecycle`, `TriggerLifecycle`

Aggregates runtime usage counters. Projects the `UsagePayload`:

```rust
pub struct UsagePayload {
    pub target: String,
    pub interval_ms: u64,
    pub cell_runs: u64,
    pub graph_runs: u64,
    pub trigger_fires: u64,
    pub total_cost_usd: f64,
    pub total_duration_ms: u64,
}
```

### 2.9 Collective Intelligence Lens

**Block**: `roko:collective-intelligence-lens@^1` or `roko:c-factor-lens@^1`
**Observes**: `AgentLifecycle`, `SignalLifecycle`, `MemoryLifecycle`

Computes the c-factor and its component metrics for a Space. Projects
the `CFactorPayload`:

```rust
pub struct CFactorPayload {
    pub space: String,
    pub interval_ms: u64,
    pub c_factor: f64,
    pub turn_taking_entropy: f64,
    pub peer_prediction_accuracy: f64,
    pub citation_reciprocity: f64,
    pub hdc_diversity: f64,
    pub agent_count: usize,
    pub active_agents: usize,
    pub dominant_agent_share: f64,
    pub knowledge_flow_edges: usize,
    pub avg_agent_vitality: f64,
}
```

Four pure calculation helpers support this Lens:
- `turn_taking_entropy(turns_per_agent)` -- normalized Shannon entropy
- `peer_prediction_accuracy(predictions, outcomes)` -- 1 - MSE
- `citation_reciprocity(survived, total)` -- weighted fraction
- `hdc_diversity(similarities)` -- 1 - mean pairwise HDC similarity

---

## 3. Derived Lenses

### 3.1 Trend Lens

**Block**: `roko:trend-lens@^1`
**Observes**: `SignalLifecycle` (receives upstream Lens output as Signals)
**Scope**: `lens:{upstream-name}`

Computes statistical trends from another Lens's output over a
configurable window. Projects the `TrendPayload`:

```rust
pub struct TrendPayload {
    pub source_lens: String,
    pub metric: String,
    pub window_ms: u64,
    pub slope: f64,
    pub ema: f64,
    pub ema_previous: f64,
    pub direction: TrendDirection,  // Rising | Falling | Stable
    pub r_squared: f64,
    pub data_points: usize,
}
```

### 3.2 Anomaly Lens

**Block**: `roko:anomaly-lens@^1`
**Observes**: `SignalLifecycle`
**Scope**: `lens:{upstream-name}`

Detects outliers in another Lens's output. Projects the
`AnomalyPayload`:

```rust
pub struct AnomalyPayload {
    pub source_lens: String,
    pub metric: String,
    pub observed_value: f64,
    pub expected_value: f64,
    pub deviation: f64,
    pub direction: AnomalyDirection,  // Above | Below
    pub severity: AnomalyLevel,       // Moderate | Severe | Critical
}
```

---

## 4. Chaining

Derived Lenses use `LensScope::Lens("upstream-name")` to declare
their upstream dependency. The `LensRegistry` enforces:

- No cycles (topological sort with deterministic cycle reporting)
- Outputs route only to direct downstream consumers (transitive
  consumers wait for their immediate upstream)
- Chain order is deterministic and dependency-first

Example chain: `cost -> trend -> anomaly`

```toml
[[lenses]]
name = "cost"
block = "roko:cost-lens@^1"
scope = "graph"

[[lenses]]
name = "cost-trend"
block = "roko:trend-lens@^1"
scope = "lens:cost"

[[lenses]]
name = "cost-anomaly"
block = "roko:anomaly-lens@^1"
scope = "lens:cost-trend"
```

---

## 5. Metric-Oriented Lens Instances

Three concrete `Lens` implementations project subsets of the
`MetricRegistry`:

| Lens | Metric families | Labels |
|------|----------------|--------|
| `TokenUsageLens` | `roko_llm_tokens_total` | `provider`, `model`, `direction` |
| `LatencyLens` | `roko_llm_ttft_seconds`, `roko_llm_request_duration_seconds` | `provider`, `model` |
| `CostLens` | `roko_llm_cost_usd_total` | `provider`, `model` |

These are registered as the default set via `default_registry()`:
`"token-usage"`, `"latency"`, `"cost"`.

---

## 6. Source References

| File | What it contains |
|------|------------------|
| `crates/roko-core/src/telemetry_observe.rs` | All 11 event-oriented payload structs, `ErrorCategory`, `AlertLevel`, `TrendDirection`, `AnomalyDirection`, `AnomalyLevel`, pure calculation helpers |
| `crates/roko-core/src/obs/lens.rs` | `TokenUsageLens`, `LatencyLens`, `CostLens`, `CollectorLens` |
| `crates/roko-core/src/lens_registry.rs` | `observes_for_block` (event-family defaults per block) |

---

## Verification

```bash
# Event-family routing per block
cargo test -p roko-core stacking_routes_in_registration_order

# Chain ordering
cargo test -p roko-core chain_order_is_dependency_first_and_deterministic

# Metric-oriented lens filtering
cargo test -p roko-core token_usage_lens_filters_to_token_counters_only
cargo test -p roko-core latency_lens_filters_to_latency_histograms_only
cargo test -p roko-core cost_lens_filters_to_cost_counters_only
```
