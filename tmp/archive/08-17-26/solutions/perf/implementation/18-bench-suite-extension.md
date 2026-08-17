# 18 — Bench Suite Extensions (consistency, cost, Pareto)

> Extends roko's built-in `bench` infrastructure with three things HAL
> teaches us are valuable: K-trial consistency, real cost tracking, and
> cost/quality Pareto reporting.
>
> Effort: 8-12 h. Risk: low (additive features on existing harness).

---

## Goal & success criteria

After this change:

1. `SweBenchOptions` carries a `trials: usize` field. Each instance is
   re-run K times and consistency metrics (k-pass rate, sequence
   variance) are computed.
2. `BenchResult.cost_usd` is wired to the actual `roko_learn::costs_db`
   instead of staying at `0.0`.
3. A new `roko bench compare` subcommand reads two
   `BenchRunResult` files and computes per-task deltas + a Pareto
   frontier across runs.
4. Bench results land in `.roko/bench/perf/` in a stable JSONL format
   that CI can diff.

Done when:

- `roko bench swe-mini --trials 5` completes a full 5-trial pass.
- The output JSON contains `consistency.{k_pass_rate,
  distribution_consistency, sequence_consistency}` per task.
- `roko bench compare baseline.json pr.json --threshold 20` exits
  non-zero when any task regresses by >20 %.
- A nightly CI workflow runs `roko bench swe-mini` against `main`
  and persists the result to a known location for diffing.

---

## Background

- Source: `HAL-AND-AGENT-BENCHMARKS.md` §6 (reliability metrics),
  §8 (proposed bench suite), `HAL-BENCHMARK-INTEGRATION.md` §4.
- Existing infra:
  - `crates/roko-cli/src/bench.rs` — SWE-bench proxy harness.
  - `crates/roko-serve/src/bench.rs` — `BenchSuite` / `BenchTask`
    types.
  - `crates/roko-cli/src/bench_demo.rs` — naive vs optimized comparison.
  - `crates/roko-learn/src/costs_db.rs` — cost tracking.

---

## Files to read first

| File | Why |
|---|---|
| `crates/roko-cli/src/bench.rs` | Primary edit site for trials + cost wiring. |
| `crates/roko-learn/src/costs_db.rs` | Source of truth for cost. |
| `crates/roko-serve/src/bench.rs` | `BenchResult` type; ensure the new fields serialise compatibly. |
| `crates/roko-cli/src/commands/mod.rs` | Where to add the `bench compare` subcommand. |

---

## Code-level plan

### Step 1 — Add `trials` to `SweBenchOptions`

```rust
// crates/roko-cli/src/bench.rs
#[derive(Debug, Clone)]
pub struct SweBenchOptions {
    // ... existing fields ...
    /// Number of times to re-run each instance for consistency metrics.
    /// 1 = no re-run. Default: 1.
    pub trials: usize,
    /// Optional pareto-frontier output path.
    pub pareto_out: Option<PathBuf>,
}
```

Defaults preserved for callers that don't set the new fields.

### Step 2 — Multi-trial runner

```rust
async fn run_instance_with_trials(
    instance: &SweBenchInstance,
    options: &SweBenchOptions,
    trials: usize,
) -> InstanceTrialReport {
    let mut runs = Vec::with_capacity(trials);
    for trial_idx in 0..trials {
        let result = run_task_real(instance, options, trial_idx).await;
        runs.push(result);
    }
    InstanceTrialReport {
        instance_id: instance.id.clone(),
        runs,
        consistency: compute_consistency(&runs),
    }
}

#[derive(Debug, Serialize)]
pub struct ConsistencyMetrics {
    pub trials: usize,
    pub passes: usize,
    pub k_pass_rate: f64,                   // passes / trials
    pub all_pass: bool,                     // strict all-K
    pub distribution_consistency: f64,      // tool-name distribution Jaccard
    pub sequence_consistency: f64,          // edit-distance over action ordering
    pub cost_cv: f64,                       // coefficient of variation across trials
    pub duration_cv: f64,
}

fn compute_consistency(runs: &[BenchResult]) -> ConsistencyMetrics {
    let trials = runs.len();
    let passes = runs.iter().filter(|r| r.passed).count();
    // k_pass_rate, all_pass straightforward.
    // distribution_consistency: collect tool-name multisets per run, compute pairwise Jaccard, average.
    // sequence_consistency: collect action sequences, compute normalised Levenshtein, average.
    // cost_cv = stddev / mean.
    // ...
    ConsistencyMetrics { /* ... */ }
}
```

### Step 3 — Wire real cost into `run_task_real`

`crates/roko-cli/src/bench.rs:610` (or wherever `run_task_real` lives)
currently leaves `cost_usd: 0.0`. Replace with:

```rust
let cost_usd = costs_db.cost_for_run(&run_id).await.unwrap_or(0.0);
```

`costs_db` is an `Arc<CostsDb>` — pass it down from the bench
runner's caller. The DB already records per-run cost in
`runtime_feedback::record_completed_run`.

### Step 4 — Add `roko bench compare`

```rust
// crates/roko-cli/src/commands/bench_compare.rs (NEW)

#[derive(clap::Args)]
pub struct BenchCompareArgs {
    #[clap(value_name = "BASELINE")]
    pub baseline: PathBuf,
    #[clap(value_name = "CANDIDATE")]
    pub candidate: PathBuf,
    /// Fail if any task regresses by more than N percent.
    #[clap(long, default_value = "20")]
    pub threshold: f64,
    /// Print a Pareto frontier comparison.
    #[clap(long)]
    pub pareto: bool,
}

pub async fn run_bench_compare(args: BenchCompareArgs) -> anyhow::Result<()> {
    let baseline: BenchRunResult = read_json(&args.baseline)?;
    let candidate: BenchRunResult = read_json(&args.candidate)?;

    println!("{:^60}", "Per-task deltas");
    println!("{:<30} {:>12} {:>12} {:>10}", "task", "baseline_ms", "candidate_ms", "delta%");
    let mut regressions = Vec::new();
    for c in &candidate.tasks {
        if let Some(b) = baseline.tasks.iter().find(|b| b.id == c.id) {
            let delta = ((c.duration_ms as f64 - b.duration_ms as f64) / b.duration_ms as f64) * 100.0;
            println!("{:<30} {:>12} {:>12} {:>9.1}%", c.id, b.duration_ms, c.duration_ms, delta);
            if delta > args.threshold {
                regressions.push((c.id.clone(), delta));
            }
        }
    }

    if args.pareto {
        print_pareto(&baseline, &candidate);
    }

    if !regressions.is_empty() {
        eprintln!("\n{} regression(s) > {}%:", regressions.len(), args.threshold);
        for (id, d) in &regressions {
            eprintln!("  {id}: +{d:.1}%");
        }
        std::process::exit(1);
    }
    Ok(())
}
```

### Step 5 — Persist results to a stable layout

```text
.roko/bench/perf/
  YYYYMMDD-HHMMSS/
    suite-results.json          # serialised BenchRunResult
    consistency.json            # per-instance ConsistencyMetrics
    pareto.json                 # cost vs quality Pareto curve
    summary.md                  # human-readable digest
  latest -> YYYYMMDD-HHMMSS/    # symlink updated atomically
```

Use `roko_fs::atomic::atomic_write_json` for each file. The `latest`
symlink is updated via a temp-symlink-rename pattern.

---

## Step-by-step execution

1. `git checkout -b perf/18-bench-suite-extension`.
2. Add `trials` to `SweBenchOptions` (Step 1).
3. Add multi-trial runner + consistency metrics (Step 2).
4. Wire cost from `costs_db` (Step 3).
5. Add `bench compare` subcommand (Step 4).
6. Persist results to stable layout (Step 5).
7. Add CI workflow that runs `roko bench swe-mini --trials 3` nightly.
8. PR `feat(bench): K-trial consistency + cost + compare subcommand`.

---

## Anti-patterns / things NOT to do

- **Do NOT default `trials` to anything other than 1.** Multi-trial
  multiplies API spend; it must be opt-in.
- **Do NOT compare across different model configs** in `bench compare`.
  Add a sanity check: if `baseline.config_hash != candidate.config_hash`,
  warn the user and require `--force`.
- **Do NOT compute `distribution_consistency` over an empty action
  set.** A failed run has no actions; treat as 0.0 with a flag, not
  NaN.
- **Do NOT use `f64::NAN` anywhere in serialised output.** It breaks
  most JSON parsers and downstream tooling. Use `Option<f64>` and emit
  `null`.
- **Do NOT trust the `costs_db` to always have an entry for the run.**
  Fast-fail bench runs may not flush; default to 0.0 with a metric
  counter (`bench_cost_missing_total`) so we can spot the regression.
- **Do NOT include the entire run-id chain in the summary.** The
  per-task summary should be ≤80 cols; long IDs make grep painful.
- **Do NOT auto-update the leaderboard from CI.** Nightly results land
  in a known location; promotion to the public leaderboard is a manual
  step (different audit trail).
- **Do NOT bundle this with HAL integration (Plan 17).** They are
  related but independent; review them separately.

---

## Test plan

```rust
#[tokio::test]
async fn multi_trial_consistency_computes_k_pass_rate() {
    let runs = vec![passing_run(), passing_run(), failing_run()];
    let m = compute_consistency(&runs);
    assert_eq!(m.passes, 2);
    assert!((m.k_pass_rate - 2.0/3.0).abs() < 1e-6);
    assert!(!m.all_pass);
}

#[tokio::test]
async fn cost_is_wired_from_costs_db() {
    let dir = tempfile::tempdir().unwrap();
    let costs_db = CostsDb::open_under(dir.path()).await.unwrap();
    costs_db.record(/* run_id */ "r1", 0.42).await.unwrap();
    let opts = SweBenchOptions { trials: 1, /* ... */ };
    let result = run_instance(&dummy_instance("r1"), &opts, &costs_db).await;
    assert!((result.cost_usd - 0.42).abs() < 1e-6);
}

#[test]
fn bench_compare_fails_on_regression() {
    let baseline = bench_run(&[("t1", 100), ("t2", 200)]);
    let candidate = bench_run(&[("t1", 130), ("t2", 200)]);   // t1 +30%
    let regressions = compute_regressions(&baseline, &candidate, 20.0);
    assert_eq!(regressions, vec![("t1".into(), 30.0)]);
}
```

---

## Rollback plan

- All additions are opt-in (`--trials > 1`, `bench compare` is a new
  subcommand). Reverting removes them with no behaviour impact.
- Persisted bench artifacts under `.roko/bench/perf/` are leaf data;
  delete to roll back state.

---

## Status check (acceptance)

- [ ] `SweBenchOptions.trials` exists; multi-trial runs produce
      `ConsistencyMetrics`.
- [ ] `BenchResult.cost_usd` is non-zero when `costs_db` has data.
- [ ] `roko bench compare` exists and exits non-zero on regression.
- [ ] Stable result layout under `.roko/bench/perf/` with `latest`
      symlink.
- [ ] Nightly CI workflow exists and writes results.
