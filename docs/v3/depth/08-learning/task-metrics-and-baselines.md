# 08-learning/07 -- Task Metrics and Baselines

> Per-gate metrics, per-slice baseline computation, efficiency events with
> prompt-level attribution, and A-D letter grading for prompt assembly.

**Parent:** [08-LEARNING](../../08-LEARNING.md)

**Source:** `crates/roko-learn/src/task_metric.rs`,
`crates/roko-learn/src/baseline.rs`, `crates/roko-learn/src/efficiency.rs`

**Persistence:** `.roko/learn/task-metrics.jsonl`,
`.roko/learn/efficiency.jsonl`

**Cross-references:** [episode-logger](episode-logger.md),
[regression-detection](regression-detection.md),
[cascade-router](cascade-router.md)

---

## 1. Purpose

The task metrics and baselines subsystem provides the quantitative foundation
for all performance evaluation in Roko. Every gate execution produces one
immutable `TaskMetric` record. These records accumulate in an append-only JSONL
file, and the baseline computation groups them by `(role, complexity_band)` to
produce per-slice statistical profiles. The regression detector then compares
fresh batches against these baselines to identify performance degradation.

The efficiency module extends per-turn instrumentation with prompt-level
attribution, tool utilization tracking, and A-D letter grading.

---

## 2. TaskMetric Schema

```rust
pub struct TaskMetric {
    pub task_id: String,
    pub plan_id: String,
    pub role: String,
    pub complexity_band: String,
    pub model: String,
    pub backend: String,
    pub gate: String,
    pub gate_passed: bool,
    pub iteration: u32,
    pub cost_usd: f64,
    pub duration_ms: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub config_hash: ConfigHash,
    pub timestamp: DateTime<Utc>,
}
```

### 2.1 MetricFilter

```rust
pub struct MetricFilter {
    pub roles: HashSet<String>,
    pub complexity_bands: HashSet<String>,
    pub plan_ids: HashSet<String>,
    pub gates: HashSet<String>,
    pub models: HashSet<String>,
    pub backends: HashSet<String>,
    pub gate_passed: Option<bool>,
    pub iteration_range: Option<(u32, u32)>,
    pub min_cost_usd: Option<f64>,
    pub max_cost_usd: Option<f64>,
    pub config_hashes: HashSet<String>,
}
```

All predicates are AND-combined: a record must match every non-empty field.

### 2.2 MetricsWriter and MetricsReader

- `MetricsWriter` -- thread-safe, append-only JSONL writer. Uses
  `parking_lot::Mutex` for serialization.
- `MetricsReader` -- parse JSONL lines from bytes, tolerant of corrupted
  lines (same resilience pattern as the episode logger).

---

## 3. Baseline Computation

### 3.1 SliceBaseline

```rust
pub struct SliceBaseline {
    pub role: String,
    pub complexity_band: String,
    pub pass_rate: f64,
    pub avg_cost: f64,
    pub avg_duration_ms: f64,
    pub avg_iterations: f64,
    pub avg_input_tokens: f64,
    pub avg_output_tokens: f64,
    pub avg_cache_hit_rate: f64,
    pub n_records: usize,
}
```

### 3.2 Computation

`compute_baseline()` groups `TaskMetric` records by `(role, complexity_band)`:

```
TaskMetric records
    |
    v
Group by (role, complexity_band)
    |
    +-- ("Implementer", "standard") -> 156 records
    |       pass_rate: 0.72
    |       avg_cost: $0.83
    |       avg_duration_ms: 45,000
    |       avg_iterations: 1.4
    |
    +-- ("Implementer", "complex") -> 48 records
    |       pass_rate: 0.58
    |       avg_cost: $1.52
    |
    +-- ("Reviewer", "standard") -> 89 records
            pass_rate: 0.91
            avg_cost: $0.35
```

---

## 4. Efficiency Events

The `AgentEfficiencyEvent` provides per-turn instrumentation with 20+ fields:

```rust
pub struct AgentEfficiencyEvent {
    // -- Identity --
    pub agent_id: String,
    pub role: String,
    pub backend: String,
    pub model: String,
    pub plan_id: String,
    pub task_id: String,

    // -- Token accounting --
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,

    // -- Cost accounting --
    pub cost_usd: f64,
    pub cost_usd_without_cache: f64,

    // -- Prompt composition --
    pub prompt_sections: Vec<PromptSectionMeta>,
    pub total_prompt_tokens: u64,
    pub system_prompt_tokens: u64,

    // -- Tool utilization --
    pub tools_available: u32,
    pub tools_used: u32,
    pub tool_calls: Vec<ToolCallMeta>,

    // -- Timing --
    pub wall_time_ms: u64,
    pub time_to_first_token_ms: u64,
    pub was_warm_start: bool,
}
```

### 4.1 PromptSectionMeta

```rust
pub struct PromptSectionMeta {
    pub name: String,          // e.g. "plan_brief", "workspace_map", "playbook_hits"
    pub tokens: u64,
    pub priority: u8,          // 0 = highest
    pub was_truncated: bool,
    pub was_dropped: bool,
}
```

### 4.2 ToolCallMeta

```rust
pub struct ToolCallMeta {
    pub tool_name: String,     // e.g. "Read", "Write", "Bash"
    pub duration_ms: u64,
    pub result_tokens: u64,
    pub succeeded: bool,
}
```

---

## 5. Prompt Efficiency Grading

```rust
pub enum Grade {
    A,  // >= 0.8
    B,  // >= 0.6
    C,  // >= 0.4
    D,  // < 0.4
}
```

The composite efficiency score:

```
efficiency = 0.30 * section_utilization
           + 0.30 * token_efficiency
           + 0.20 * cache_hit_rate
           + 0.20 * tool_utilization
```

| Component | Definition |
|-----------|-----------|
| Section utilization | Fraction of included sections that contributed |
| Token efficiency | output_tokens / input_tokens |
| Cache hit rate | cache_read_tokens / input_tokens |
| Tool utilization | tools_used / tools_available |

---

## 6. Role Cost Profiles

```rust
pub struct RoleCostProfile {
    pub role: String,
    pub total_cost_usd: f64,
    pub avg_cost_per_turn: f64,
    pub avg_cost_per_success: f64,
    pub total_turns: u64,
    pub total_successes: u64,
    pub avg_input_tokens: f64,
    pub avg_output_tokens: f64,
    pub avg_cache_hit_rate: f64,
}
```

These profiles answer operational questions: "Which role is most expensive?"
"Does the warm pool save money?" "Which prompt sections drove the cost?"

---

## 7. Four Key Headline Metrics

| Metric | Definition | Baseline Target |
|--------|-----------|-----------------|
| First-attempt pass rate | % of tasks passing gates on first try | > 60% |
| Iterations per plan | Average iterations to complete a plan | < 2.0 |
| Cost per plan | Total USD spent per plan | Decreasing trend |
| Prompt tokens per spawn | Input tokens for the initial prompt | < 50K |

```rust
pub struct Headlines {
    pub total_tasks: usize,
    pub passed_tasks: usize,
    pub pass_rate: f64,
    pub total_cost_usd: f64,
    pub avg_cost_per_task: f64,
    pub avg_iterations: f64,
    pub avg_duration_ms: f64,
}
```

---

## 8. Persistence

| Artifact | Format | Path |
|----------|--------|------|
| Task metrics | JSONL | `.roko/learn/task-metrics.jsonl` |
| Efficiency events | JSONL | `.roko/learn/efficiency.jsonl` |

Both files are append-only. The `MetricsWriter` batches records in memory and
flushes periodically.
