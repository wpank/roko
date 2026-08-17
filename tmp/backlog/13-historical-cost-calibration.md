# Backlog: Historical Cost Calibration for Budget Predictor

**Status**: Backlog
**Priority**: P2
**Size**: S (1 day)
**Origin**: `tmp/architecture-archive/19-visual-composition.md` (cost projection section)

---

## Problem Statement

`BudgetPredictor` in `crates/roko-compose/src/budget_predictor.rs` is a well-designed EMA-based predictor that updates its observations via `record()` calls from the runner at task completion time. However, there is a critical gap in how it is initialized and warmed up:

**The predictor has no cold-start bootstrap from the existing efficiency log.** When `load_predictor()` returns `None` (first run, or after a clean install), the predictor falls back to `fallback_tokens = 100_000` for every task — regardless of how many episodes have already been recorded in `.roko/learn/efficiency.jsonl`. A workspace that has run 500 tasks has rich cost history but the predictor starts from scratch.

Additionally, **there is no feedback loop comparing predicted vs actual cost post-task**. The runner calls `record()` with actual token counts, but there is no reporting of prediction error (predicted budget - actual budget) over time. Without this feedback, it is impossible to know whether the predictor is converging or diverging, or whether the 20% safety margin built into `predict()` is appropriate.

The architecture document (`19-visual-composition.md`) specifies cost projection as a first-class component of the authoring system's cost estimate display. The `confidence` field in the cost estimate response (`0.65` in the example) should come from a calibrated predictor, not a constant.

---

## Proposed Solution

### Step 1: Bootstrap from `efficiency.jsonl`

Add a `BudgetPredictor::bootstrap_from_efficiency(path: &Path) -> anyhow::Result<Self>` function that:

1. Opens `.roko/learn/efficiency.jsonl` and reads records line by line.
2. For each efficiency event, extracts `role`, `complexity` (derived from token count buckets if not present directly), `domain`, `actual_tokens` (output tokens), and `success` (gate outcome).
3. Calls `self.record()` for each event in chronological order so the EMA converges on recent history.
4. Returns the initialized predictor.

The efficiency event schema (from `DashboardEvent::EfficiencyEvent`) already carries the fields needed:

```rust
// From crates/roko-core/src/dashboard_snapshot.rs
DashboardEvent::EfficiencyEvent {
    task_id,
    agent_id,
    role,         // "Implementer", "Reviewer", etc.
    model,
    input_tokens,
    output_tokens,
    cost_usd,
    gate_passed,
    duration_ms,
    ..
}
```

The `domain` field can be inferred from the task `agent_id` prefix or a `domain` field if it exists in the JSONL record (to be confirmed against the actual persisted format in `crates/roko-cli/src/runner/persist.rs`).

```rust
impl BudgetPredictor {
    /// Initialize predictor state from historical efficiency events.
    ///
    /// Reads JSONL records in order. Each line is expected to be a
    /// serialized `EfficiencyRecord` (the struct written to efficiency.jsonl).
    /// Records that cannot be parsed are skipped with a warning.
    pub fn bootstrap_from_efficiency(path: &Path) -> anyhow::Result<Self> {
        let mut predictor = Self::default();
        if !path.exists() {
            return Ok(predictor);
        }
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        for line in reader.lines() {
            let Ok(line) = line else { continue };
            let Ok(record) = serde_json::from_str::<EfficiencyRecord>(&line) else {
                tracing::warn!("skipping malformed efficiency record");
                continue;
            };
            let features = TaskFeatures {
                role: record.role.clone(),
                complexity: classify_complexity(record.output_tokens),
                domain: record.domain.unwrap_or_else(|| "unknown".into()),
            };
            predictor.record(&features, record.output_tokens, record.gate_passed);
        }
        Ok(predictor)
    }
}

/// Classify token count into a complexity band.
fn classify_complexity(tokens: u64) -> String {
    match tokens {
        0..=20_000   => "trivial".into(),
        20_001..=80_000 => "standard".into(),
        _            => "complex".into(),
    }
}
```

### Step 2: Call `bootstrap_from_efficiency` at startup

In the runner startup path (`crates/roko-cli/src/runner/event_loop.rs`), replace the current `load_predictor()` cold-start with:

```rust
let predictor = load_predictor(&learn_dir)?
    .unwrap_or_else(|| {
        BudgetPredictor::bootstrap_from_efficiency(&efficiency_path)
            .unwrap_or_default()
    });
```

This means: if a persisted predictor JSON exists, use it (it already has all prior observations folded in). If not, bootstrap from the raw JSONL so the first run after a clean install is not cold.

### Step 3: Prediction error tracking

Add a `PredictionCalibration` struct that records, per task, the ratio of predicted-to-actual tokens:

```rust
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PredictionCalibration {
    /// (predicted, actual) pairs in chronological order.
    history: VecDeque<(u64, u64)>,
    /// Maximum history length.
    capacity: usize,
}

impl PredictionCalibration {
    pub fn record(&mut self, predicted: u64, actual: u64) {
        if self.history.len() >= self.capacity {
            self.history.pop_front();
        }
        self.history.push_back((predicted, actual));
    }

    /// Mean absolute percentage error over recent history.
    pub fn mape(&self) -> f64 {
        if self.history.is_empty() { return 0.0; }
        let sum: f64 = self.history.iter().map(|(p, a)| {
            if *a == 0 { 0.0 } else { (*p as f64 - *a as f64).abs() / *a as f64 }
        }).sum();
        sum / self.history.len() as f64
    }

    /// Confidence score: 1.0 - mape, clamped to [0.0, 1.0].
    pub fn confidence(&self) -> f64 {
        (1.0 - self.mape()).clamp(0.0, 1.0)
    }
}
```

The runner records `(predicted_budget, actual_tokens)` at task completion, and the `confidence()` value is used in cost estimate responses (surfaced via `roko-compose`'s cost projection API) and logged in `efficiency.jsonl` as an optional `prediction_error_pct` field.

### Step 4: Expose calibration in `roko learn`

Add a new sub-command `roko learn budget-calibration` (or extend `roko learn efficiency`) that reads `.roko/learn/budget-predictor.json` and prints:

```
Budget predictor calibration report
  Observations:  312 feature keys
  Recent MAPE:   18.4%
  Confidence:    0.82
  Top over-predicted roles:
    Reviewer:complex:code  predicted=95k actual=42k (+126%)
  Top under-predicted roles:
    Implementer:complex:chain  predicted=60k actual=134k (-55%)
```

---

## Implementation Location

| Component | Path |
|---|---|
| `bootstrap_from_efficiency()` | `crates/roko-compose/src/budget_predictor.rs` |
| `PredictionCalibration` struct | `crates/roko-compose/src/budget_predictor.rs` |
| Startup wiring | `crates/roko-cli/src/runner/event_loop.rs` |
| Calibration CLI sub-command | `crates/roko-cli/src/commands/learn.rs` |
| Efficiency record schema | `crates/roko-cli/src/runner/persist.rs` (confirm fields) |

---

## Acceptance Criteria

1. After running `cargo test --workspace`, `BudgetPredictor::bootstrap_from_efficiency()` correctly initializes a predictor from a fixture JSONL file: given 50 synthetic efficiency records with known token counts, the predictor's `observation_count()` equals the number of distinct feature keys in the fixture and `predict()` returns values derived from those observations rather than `fallback_tokens`.

2. On first runner startup in a workspace that has an existing `efficiency.jsonl` with at least 10 records, `predict()` returns a non-fallback value for the feature key matching the most common role/domain in the JSONL (verified by a unit test over a fixture path).

3. `PredictionCalibration::mape()` returns 0.0 for a history of perfect predictions (predicted == actual) and 1.0 for a history where every prediction is double the actual.

4. `PredictionCalibration::confidence()` is exposed in the cost estimate object returned by the composition layer; the JSON field `"confidence"` is a float in `[0.0, 1.0]` (not a hardcoded constant).

5. `roko learn` (or the extended sub-command) prints a calibration summary including observation count, recent MAPE, and confidence; `cargo run -p roko-cli -- learn` exits 0.

---

## References

- Source spec: `/Users/will/dev/nunchi/roko/roko/tmp/architecture-archive/19-visual-composition.md` (cost projection section)
- Existing implementation: `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/budget_predictor.rs`
- Efficiency log path: `/Users/will/dev/nunchi/roko/roko/.roko/learn/efficiency.jsonl`
- Persist paths: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/persist.rs`
- Efficiency event schema: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/dashboard_snapshot.rs` (`DashboardEvent::EfficiencyEvent`)
- Runner emission site: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
- Learn CLI: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/learn.rs`
