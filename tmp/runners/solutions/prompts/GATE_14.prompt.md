# GATE_14: Call Hotelling observe_pipeline after full run

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-14`](../ISSUE-TRACKER.md#gate-14)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.14
- Priority: **P1**
- Effort: 2 hours
- Depends on: `GATE_02` (source 4.2), `GATE_03` (source 4.3)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_14 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`observe_pipeline()` at `crates/roko-gate/src/adaptive_threshold.rs:468` feeds the full pass-rate vector to Hotelling's T-squared detector for joint anomaly detection. `joint_anomaly_detected()` at line 488 returns whether the last observation triggered an anomaly. Neither is called from runtime code (AP-8).

## Exact Changes

1. After the verdict loop in `run_gates()`, build the pass-rate vector and call `observe_pipeline()`:
   ```rust
   let joint_anomaly = if let Some(adaptive) = &self.adaptive {
       let pass_rates: Vec<f64> = verdicts.iter()
           .filter(|v| !v.skipped)
           .map(|v| if v.passed { 1.0 } else { 0.0 })
           .collect();
       if pass_rates.len() >= 2 {
           if let Ok(mut thresholds) = adaptive.lock() {
               thresholds.observe_pipeline(&pass_rates);
               thresholds.joint_anomaly_detected()
           } else {
               false
           }
       } else {
           false
       }
   } else {
       false
   };
   ```
2. Include `joint_anomaly` in the returned `GateReport`.
3. Add a test: 50 normal observations then a joint drop to [0.0, 0.0] should set `joint_anomaly: true`.

## Write Scope

- `crates/roko-gate/src/gate_service.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] After a sudden multi-gate drop, `GateReport.joint_anomaly` is true
- [ ] Normal runs have `joint_anomaly: false`

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_14 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After a sudden multi-gate drop, `GateReport.joint_anomaly` is true
- Normal runs have `joint_anomaly: false`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_14 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
