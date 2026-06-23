# TEST_23: Benchmark regression detection

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-23`](../ISSUE-TRACKER.md#test-23)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.23
- Priority: **P1**
- Effort: 3 hours
- Depends on: `TEST_21` (source 15.21)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_23 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Uses baselines from Task 15.21's `perf_baselines.json` to detect regressions. Addresses AP-BENCH-STUB by providing the baseline infrastructure that `BenchmarkRegressionGate` needs.

## Exact Changes

1. Load baselines from `perf_baselines.json`
2. Run each performance measurement (same as Task 15.21)
3. Compare against baseline with configurable threshold (default: 20% regression)
4. Report per-measurement: name, current value, baseline, delta percentage, pass/fail
5. Test detection: artificially inflate a baseline by 50%, verify the test catches the "regression"
6. Test threshold configuration: 0% threshold catches any increase, 100% threshold catches nothing
7. Output regression report as structured JSON for CI consumption
8. Provide a `RegressionChecker` utility struct that can be reused by `BenchmarkRegressionGate`

## Design Guidance

The `RegressionChecker` struct encapsulates:
```rust
pub struct RegressionChecker {
    baselines: HashMap<String, f64>,
    threshold_pct: f64,
}
impl RegressionChecker {
    pub fn load(path: &Path) -> Result<Self>;
    pub fn check(&self, name: &str, current: f64) -> RegressionResult;
}
pub struct RegressionResult {
    pub name: String,
    pub baseline: f64,
    pub current: f64,
    pub delta_pct: f64,
    pub passed: bool,
}
```
This struct should live in the test harness crate so `BenchmarkRegressionGate` can eventually use it at runtime.

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Regression detection catches 20%+ degradation
- [ ] Threshold is configurable per measurement
- [ ] Report is machine-parseable (JSON)
- [ ] Artificial regression test verifies detection works

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_23 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Regression detection catches 20%+ degradation
- Threshold is configurable per measurement
- Report is machine-parseable (JSON)
- Artificial regression test verifies detection works
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_23 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
