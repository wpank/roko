# EVAL_12: `CriterionStats` -- per-criterion adaptive tracking

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-12`](../ISSUE-TRACKER.md#eval-12)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.12
- Priority: **P1**
- Effort: 5 hours
- Depends on: `EVAL_04` (source 5.4)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_12 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The existing `AdaptiveThresholds` at `crates/roko-gate/src/adaptive_threshold.rs` tracks per-rung statistics (EMA pass rate, consecutive passes, CUSUM). `CriterionStats` extends this concept to per-criterion granularity, enabling criterion-level skip decisions and cost tracking.

## Exact Changes

1. Define `CriterionStats`:
   ```rust
   pub struct CriterionStats {
       pub ema_pass_rate: f64,
       pub consecutive_passes: u32,
       pub cusum_high: f64,
       pub cusum_low: f64,
       pub score_history: VecDeque<f64>,  // last 50
       pub avg_duration_ms: f64,
       pub avg_cost_usd: f64,
       pub total_observations: u64,
   }
   ```
2. Implement `observe(passed: bool, score: f64, duration_ms: u64, cost_usd: f64)` with the same EMA/CUSUM logic as `AdaptiveThresholds::observe()`.
3. Implement `should_skip() -> bool` based on consecutive pass streak (threshold: 20).
4. Define `CriterionStatsStore` persisting to `.roko/eval/criterion-stats.json` with `load()` and `save()` (same pattern as `AdaptiveThresholds::load/save`).
5. Wire into `EvalService::evaluate()`: after each criterion evaluation, call `stats.observe(passed, score, duration_ms, cost_usd)`.

## Write Scope

- `crates/roko-eval/src/lib.rs`
- `crates/roko-eval/src/service.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Test that 20+ consecutive passes triggers skip suggestion
- [ ] Round-trip persistence test

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_12 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Test that 20+ consecutive passes triggers skip suggestion
- Round-trip persistence test
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_12 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
