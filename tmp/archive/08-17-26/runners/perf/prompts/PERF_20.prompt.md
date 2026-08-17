# PERF_20: Bench K-trial trials + cost wiring

## Task

Extend the native **SWE-bench / bench** harness so each instance can run
**K trials** (`--trials`), emit **consistency metrics** in JSON, and wire
`BenchResult.cost_usd` from `roko_learn::costs_db::CostsDb` instead of
hard-coded `0.0`. This batch **does not** add `roko bench compare` (that
is PERF_21).

## Tracker & sources

- Issue tracker row: [ISSUE-TRACKER.md#perf_20](../ISSUE-TRACKER.md#perf_20)
- Plan: `tmp/solutions/perf/implementation/18-bench-suite-extension.md` (§1–3, §5 partial)
- Performance contract: **C-19**
- Priority: EX
- Effort: 8–12 h (split across PERF_20 + PERF_21)
- Depends on: none
- Wave: 1

## Problem

HAL-style evaluation cares about **repeatability** (K passes) and **true
cost**. Today the bench path may run once and report `cost_usd = 0.0`
even when the learn DB recorded spend.

## Exact Changes

### Step 1 — `SweBenchOptions`

In `crates/roko-cli/src/bench.rs`:

- Add `pub trials: usize` with **`Default = 1`**.
- Thread CLI flag `--trials <N>` (clap) only on the relevant bench
  subcommand(s) (e.g. `swe-mini` / `swe` — match existing CLI structure).
- **Do not** default to K>1 (API cost).

### Step 2 — Multi-trial runner

- Add `InstanceTrialReport` (or equivalent name) holding `instance_id`,
  `runs: Vec<BenchResult>`, `consistency: ConsistencyMetrics`.
- `async fn run_instance_with_trials(...)` loops `0..trials`, calling the
  existing single-run path (`run_task_real` / current internal helper —
  discover the real symbol by reading the file).
- `ConsistencyMetrics` fields per plan / ISSUE-TRACKER:
  `trials`, `passes`, `k_pass_rate`, `all_pass`,
  `distribution_consistency`, `sequence_consistency`, `cost_cv`,
  `duration_cv`.
- Use **`Option<f64>`** for any metric undefined on small samples — **no
  `f64::NAN` in serde JSON**.

### Step 3 — `compute_consistency`

Implement deterministic, documented heuristics:

- `k_pass_rate = passes / trials` (trials ≥ 1).
- `distribution_consistency`: Jaccard over tool-name multisets between
  runs; if no tools on a failed run, use `0.0` with a documented branch
  (plan anti-pattern: no NaN).
- `sequence_consistency`: normalized edit distance / agreement metric
  over ordered tool calls — keep O(n²) acceptable for bench sizes; document.
- `cost_cv` / `duration_cv`: coefficient of variation; `None` if mean is
  0 or trials < 2.

### Step 4 — Cost wiring

- Thread `Arc<CostsDb>` (or `&CostsDb`) from the bench entrypoint down to
  where `BenchResult` is finalized.
- `cost_usd = costs_db.cost_for_run(&run_id).await.unwrap_or(0.0)` (or
  sync API if the DB is sync — match existing `costs_db` API).
- Increment / record a metric counter **`bench_cost_missing_total`** (or
  `tracing::warn!` + atomic counter) when lookup misses — plan requires
  observability.

### Step 5 — `crates/roko-serve/src/bench.rs`

- Extend `BenchResult` / `BenchSuite` types so serialized JSON includes
  optional `consistency: ...` when trials > 1.
- Preserve backward compatibility: `trials == 1` omits or nulls extra
  blocks per serde policy — document choice.

### Step 6 — Tests (`crates/roko-cli`)

- `multi_trial_consistency_computes_k_pass_rate` — construct synthetic
  `BenchResult` rows with pass/fail and known tools; assert metrics.
- `cost_is_wired_from_costs_db` — temp dir + insert cost for a run id +
  assert `cost_usd` on result (adapt to actual `CostsDb` API names).

## Write Scope

- `crates/roko-cli/src/bench.rs`
- `crates/roko-cli/src/main.rs` (only if CLI definitions live there)
- `crates/roko-serve/src/bench.rs`
- New tests in `crates/roko-cli` (module `#[cfg(test)]` in `bench.rs` or
  `tests/*.rs` per crate convention)

## Read-Only Context

- `crates/roko-learn/src/costs_db.rs`
- `crates/roko-learn/src/runtime_feedback.rs`
- `tmp/solutions/perf/implementation/18-bench-suite-extension.md`

## Acceptance Criteria

- [ ] `SweBenchOptions.trials` exists; default 1.
- [ ] K-trial loop + `ConsistencyMetrics` in output JSON when trials > 1.
- [ ] Optional fields use `Option<f64>` / `skip_serializing_if` — no NaN JSON.
- [ ] `BenchResult.cost_usd` reflects `CostsDb` when data exists.
- [ ] Missing-cost path increments `bench_cost_missing_total` (or approved equivalent).
- [ ] Tests `multi_trial_consistency_computes_k_pass_rate` and `cost_is_wired_from_costs_db` pass.
- [ ] **No** `bench compare` subcommand in this PR (PERF_21).
- [ ] Commit message trailer: `tracker: PERF_20 done <sha>`.

## Verify

```bash
rg -n 'trials|ConsistencyMetrics|cost_for_run|bench_cost_missing' crates/roko-cli/src/bench.rs crates/roko-serve/src/bench.rs crates/roko-learn/src/costs_db.rs
```

## Do NOT

- Do NOT default `--trials` above 1.
- Do NOT emit NaN in JSON.
- Do NOT compare runs across different configs (no compare cmd here).
- Do NOT bundle HAL changes (PERF_19).
- Do NOT compile or run tests during the batch (`00-RULES.md`).

## Tracker update

```
tracker: PERF_20 done <commit-sha>
```
