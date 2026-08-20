# 13 — Historical Cost Calibration for Budget Predictor

**Priority**: P2 — prevents cold-start prediction failures on existing workspaces; required for accurate cost projection in the composition layer
**Size**: S (1 day)
**Crates**: `crates/roko-compose/` (primary), `crates/roko-cli/` (wiring), `crates/roko-learn/` (efficiency record type)
**Depends on**: None

---

## Background

Roko executes LLM agent tasks in sequence or in parallel. Before dispatching an agent, it predicts how many tokens the task will consume so it can set an appropriate budget (token cap) for the API call. Over-allocating wastes money; under-allocating causes the agent to hit the cap mid-task and produce incomplete output.

The predictor that performs this estimation is `BudgetPredictor` in `crates/roko-compose/src/budget_predictor.rs`. It works by keeping an exponential moving average (EMA) of actual token usage, keyed by a `TaskFeatures` triple of `(role, complexity, domain)`. For example, after seeing an `Implementer:standard:code` task use 80,000 tokens, it predicts 96,000 tokens (80,000 × 1.2 safety margin) for the next such task.

The problem: when the predictor has no history for a feature key, it falls back to a hard-coded `fallback_tokens = 100_000`. A workspace that has run 500 tasks over multiple sessions already has rich history written to `.roko/learn/efficiency.jsonl`, but every new runner session discards this and starts from scratch. The predictor only warms up during the current run.

This item adds:
1. A bootstrap function that initializes the predictor from `efficiency.jsonl` on first load.
2. A `PredictionCalibration` struct that tracks prediction accuracy over time (MAPE).
3. A `roko learn` sub-command to display the calibration report.

## Current State

1. **`BudgetPredictor` struct** — fully implemented at `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/budget_predictor.rs` lines 94–220. Has `predict()`, `record()`, `observation_count()`, and `has_history()` methods.
2. **`load_predictor()` function** — at `budget_predictor.rs` lines 408–417. Returns `Ok(None)` when `budget-predictor.json` does not exist (cold start). When `None`, the runner currently has no fallback to bootstrap from history.
3. **`fallback_tokens` constant** — default value `100_000`, defined at `budget_predictor.rs` line 127. Used when `predict()` finds no matching key and no partial matches.
4. **Efficiency log location** — `.roko/learn/efficiency.jsonl`. In the runner startup, the path is constructed at `event_loop.rs` line 2607: `config.layout.learn_dir().join("efficiency.jsonl")`.
5. **`AgentEfficiencyEvent` struct** — defined in `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/efficiency.rs` lines 80–169. Fields relevant here: `role` (line 85), `output_tokens` (line 105), `gate_passed` (line 151), `plan_id` (line 91), `task_id` (line 93). The `domain` field is not directly present; it can be inferred from `plan_id` prefix or defaulted to `"code"`.
6. **Runner startup path** — predictor is loaded in `event_loop.rs` around line 2607 via `efficiency_path`. The existing `load_predictor()` call and its cold-start behavior need to be identified and extended.
7. **`LearnCmd` enum** — defined at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` line 1171. Variants: `All`, `Route`, `Experiments`, `Efficiency`, `Episodes`, `Tune`. The handler dispatches to `dispatch_learn()` in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/learn.rs`.
8. **`TaskFeatures` struct** — at `budget_predictor.rs` lines 41–71. Constructor `TaskFeatures::new(role, complexity, domain)` takes string-like arguments.

## Implementation Plan

### Step 1: Add `bootstrap_from_efficiency()` to `BudgetPredictor`

In `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/budget_predictor.rs`, add the following after the `load_predictor` function (around line 417):

```rust
/// Initialize predictor state from historical efficiency events in `path`.
///
/// Reads JSONL records in chronological order and calls `record()` for each
/// valid event. Records that cannot be parsed as `AgentEfficiencyEvent` are
/// skipped with a `tracing::warn!`. If `path` does not exist, returns a
/// default (empty) predictor.
///
/// # Errors
///
/// Returns an error only if the file exists but cannot be opened (permission
/// errors, etc.). Parse failures are non-fatal.
pub fn bootstrap_from_efficiency(path: &std::path::Path) -> std::io::Result<Self> {
    use std::io::BufRead;

    if !path.exists() {
        return Ok(Self::default());
    }

    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut predictor = Self::default();
    let mut line_count = 0u64;
    let mut skipped = 0u64;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => { skipped += 1; continue; }
        };
        if line.trim().is_empty() {
            continue;
        }

        // AgentEfficiencyEvent is the type written to efficiency.jsonl.
        // We import it here from roko_learn to avoid a circular dependency;
        // roko-compose already depends on roko-learn (check Cargo.toml first —
        // if not, add it there as a dev-dependency only, or use serde_json::Value
        // to extract the three needed fields manually).
        let value: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => { skipped += 1; continue; }
        };

        let role = value.get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let output_tokens = value.get("output_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let gate_passed = value.get("gate_passed")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        if output_tokens == 0 {
            skipped += 1;
            continue;
        }

        let complexity = classify_complexity(output_tokens);
        // domain: prefer explicit field; fall back to "code" for backwards compat
        let domain = value.get("domain")
            .and_then(|v| v.as_str())
            .unwrap_or("code")
            .to_string();

        let features = TaskFeatures::new(role, complexity, domain);
        predictor.record(&features, output_tokens, gate_passed);
        line_count += 1;
    }

    tracing::debug!(
        path = %path.display(),
        loaded = line_count,
        skipped,
        "bootstrapped BudgetPredictor from efficiency log"
    );
    Ok(predictor)
}
```

Also add the helper function (in the same file, as a private function):

```rust
/// Classify output token count into a complexity band matching TaskFeatures.
fn classify_complexity(output_tokens: u64) -> String {
    match output_tokens {
        0..=20_000       => "trivial".to_string(),
        20_001..=80_000  => "standard".to_string(),
        _                => "complex".to_string(),
    }
}
```

Note: `roko-compose` may not have `roko-learn` as a dependency. If the `Cargo.toml` for `roko-compose` (`/Users/will/dev/nunchi/roko/roko/crates/roko-compose/Cargo.toml`) does not include `roko-learn`, use `serde_json::Value` extraction as shown above rather than importing `AgentEfficiencyEvent` directly. Verify with:

```bash
grep "roko-learn" /Users/will/dev/nunchi/roko/roko/crates/roko-compose/Cargo.toml
```

### Step 2: Add `PredictionCalibration` struct

In the same `budget_predictor.rs` file, after the `BudgetPredictor` impl block (after line ~220), add:

```rust
/// Tracks prediction accuracy history for calibration reporting.
///
/// Records (predicted, actual) token pairs in a bounded ring buffer.
/// Used to compute MAPE and a confidence score for cost estimate responses.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PredictionCalibration {
    /// Chronological (predicted, actual) token pairs.
    history: std::collections::VecDeque<(u64, u64)>,
    /// Maximum number of pairs to retain. Default: 200.
    #[serde(default = "default_calibration_capacity")]
    capacity: usize,
}

fn default_calibration_capacity() -> usize { 200 }

impl PredictionCalibration {
    /// Create a calibration tracker with default capacity (200 samples).
    pub fn new() -> Self {
        Self {
            history: std::collections::VecDeque::new(),
            capacity: default_calibration_capacity(),
        }
    }

    /// Record one (predicted, actual) pair.
    pub fn record(&mut self, predicted: u64, actual: u64) {
        if self.history.len() >= self.capacity {
            self.history.pop_front();
        }
        self.history.push_back((predicted, actual));
    }

    /// Mean absolute percentage error over retained history.
    ///
    /// Returns 0.0 if history is empty.
    pub fn mape(&self) -> f64 {
        if self.history.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.history.iter().map(|(p, a)| {
            if *a == 0 {
                0.0
            } else {
                (*p as f64 - *a as f64).abs() / *a as f64
            }
        }).sum();
        sum / self.history.len() as f64
    }

    /// Confidence score: 1.0 − mape, clamped to [0.0, 1.0].
    pub fn confidence(&self) -> f64 {
        (1.0 - self.mape()).clamp(0.0, 1.0)
    }

    /// Number of (predicted, actual) pairs in the history.
    pub fn sample_count(&self) -> usize {
        self.history.len()
    }
}
```

### Step 3: Wire `bootstrap_from_efficiency` into runner startup

Find where `load_predictor` is called in the runner. Search:

```bash
grep -n "load_predictor\|BudgetPredictor" /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs | head -20
```

Once located, change the cold-start path so that if `load_predictor` returns `None`, it falls back to `bootstrap_from_efficiency`:

```rust
// Before (approximate):
let predictor = load_predictor(&learn_dir)?.unwrap_or_default();

// After:
let predictor = load_predictor(&learn_dir)?
    .unwrap_or_else(|| {
        let efficiency_path = learn_dir.join("efficiency.jsonl");
        roko_compose::budget_predictor::BudgetPredictor::bootstrap_from_efficiency(
            &efficiency_path,
        )
        .unwrap_or_default()
    });
```

### Step 4: Record prediction vs. actual at task completion

After each task completes and `record()` is called on the predictor with the actual tokens, also call `calibration.record(predicted_budget, actual_tokens)`. The `predicted_budget` must be captured before dispatch and threaded through to the completion handler.

This requires:
- Storing the predicted budget in the task attempt state (search for where `predict()` is currently called — likely in the dispatch path).
- Passing it to the completion handler where `record()` is called.
- Calling `calibration.record()` at the same site.

### Step 5: Add `BudgetCalibration` sub-command to `roko learn`

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs`, add a new variant to `LearnCmd` (at line 1171):

```rust
/// Show budget predictor calibration report.
BudgetCalibration {
    /// Working directory (default: cwd).
    #[arg(long)]
    workdir: Option<PathBuf>,
},
```

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/learn.rs`, add a match arm for the new variant:

```rust
LearnCmd::BudgetCalibration { workdir } => {
    let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
    cmd_learn_budget_calibration(&wd).await
}
```

Add the `cmd_learn_budget_calibration` function in the same file:

```rust
async fn cmd_learn_budget_calibration(workdir: &std::path::Path) -> Result<i32> {
    use roko_compose::budget_predictor::{load_predictor, load_influence};

    let learn_dir = workdir.join(".roko").join("learn");
    let predictor_path = learn_dir.clone();

    match load_predictor(&predictor_path) {
        Ok(Some(predictor)) => {
            println!("Budget predictor calibration report");
            println!("  Observations: {} feature keys", predictor.observation_count());
            // If PredictionCalibration is serialized separately:
            let cal_path = learn_dir.join("budget-calibration.json");
            if cal_path.exists() {
                if let Ok(data) = std::fs::read_to_string(&cal_path) {
                    if let Ok(cal) = serde_json::from_str::<roko_compose::budget_predictor::PredictionCalibration>(&data) {
                        println!("  Samples:      {}", cal.sample_count());
                        println!("  Recent MAPE:  {:.1}%", cal.mape() * 100.0);
                        println!("  Confidence:   {:.2}", cal.confidence());
                    }
                }
            } else {
                println!("  (No calibration history yet — run some tasks first)");
            }
        }
        Ok(None) => {
            let efficiency_path = learn_dir.join("efficiency.jsonl");
            if efficiency_path.exists() {
                let predictor = roko_compose::budget_predictor::BudgetPredictor::bootstrap_from_efficiency(
                    &efficiency_path,
                ).unwrap_or_default();
                println!("Budget predictor calibration report (bootstrapped from efficiency log)");
                println!("  Observations: {} feature keys", predictor.observation_count());
            } else {
                println!("No budget predictor data found. Run `roko plan run` to generate history.");
            }
        }
        Err(e) => {
            eprintln!("Error loading budget predictor: {e}");
            return Ok(1);
        }
    }
    Ok(0)
}
```

## Acceptance Criteria

1. A unit test passes in `budget_predictor.rs`: given a fixture JSONL file with 50 synthetic `AgentEfficiencyEvent` lines (each with `role`, `output_tokens`, `gate_passed` fields), `BudgetPredictor::bootstrap_from_efficiency(&fixture_path)` returns a predictor whose `observation_count()` equals the number of distinct `(role, complexity, domain)` keys in the fixture.
2. After bootstrapping from a fixture with at least one record for `role="Implementer"` with `output_tokens=60_000`, `predict(&TaskFeatures::new("Implementer", "standard", "code"))` returns a value greater than `fallback_tokens` (100,000 is the fallback — the bootstrapped EMA of 60,000 × 1.2 = 72,000 is smaller than the fallback, which is correct behavior since it represents real data).
3. `PredictionCalibration::mape()` returns `0.0` for a history where every (predicted, actual) pair has identical values.
4. `PredictionCalibration::mape()` returns `1.0` for a history where every predicted value is double the actual (|2x - x| / x = 1.0).
5. `PredictionCalibration::confidence()` returns a value in `[0.0, 1.0]` for all valid inputs.
6. `cargo run -p roko-cli -- learn budget-calibration` exits 0 and prints a report (or an informative message if no data exists).
7. `cargo test --workspace` passes with no regressions.

## Verification Checklist

- [ ] Run `cargo build -p roko-compose` — should compile with the new functions
- [ ] Run `cargo test -p roko-compose` — existing tests plus new unit tests should pass
- [ ] Write a test fixture JSONL file with known records and run `bootstrap_from_efficiency` against it, asserting `observation_count()` and a specific `predict()` value
- [ ] Run `PredictionCalibration` unit tests for MAPE edge cases (empty, perfect, double)
- [ ] Run `cargo run -p roko-cli -- learn budget-calibration` in the workspace root
- [ ] Verify the command exits 0 and prints a report (or "No data found" message)
- [ ] Run `cargo run -p roko-cli -- learn` and confirm `budget-calibration` appears in the sub-command list
- [ ] Run `cargo test --workspace` to confirm no regressions

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/budget_predictor.rs` | Add `bootstrap_from_efficiency()`, `classify_complexity()`, and `PredictionCalibration` struct |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` | Wire `bootstrap_from_efficiency` into cold-start path; record `PredictionCalibration` at task completion |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` | Add `BudgetCalibration` variant to `LearnCmd` enum (line 1171) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/learn.rs` | Add match arm and `cmd_learn_budget_calibration` function |
