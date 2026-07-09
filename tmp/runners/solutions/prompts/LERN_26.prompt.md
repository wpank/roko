# LERN_26: Wire Calibration Policy Loop

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-26`](../ISSUE-TRACKER.md#lern-26)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.26
- Priority: **P3**
- Effort: 4 hours
- Depends on: `LERN_09` (source 7.9)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_26 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`CalibrationPolicy` (at `calibration_policy.rs`) tracks predict-publish-correct cycles. `process_event()` (line 87) takes an `AgentEvent` and returns `Option<CalibrationCorrection>`. The `CalibrationTracker` maintains predicted vs. actual success rates.

## Exact Changes

1. At run initialization, create `CalibrationPolicy::new()` (with optional `with_bias_threshold()` and `with_min_samples()`).
2. Before dispatch, publish predicted success probability from `CascadeRouter` UCB score.
3. After gate result, record actual outcome.
4. Compute calibration error: `|predicted - actual|`.
5. Feed calibration error into the CascadeRouter's alpha schedule: high error -> higher exploration (more alpha).
6. Persist calibration state to `.roko/learn/calibration.json`.

## Write Scope

- `crates/roko-cli/src/run.rs`
- `crates/roko-learn/src/calibration_policy.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Calibration corrections are computed after sufficient samples
- [ ] High calibration error increases exploration (higher alpha)
- [ ] `roko learn all` shows calibration metrics

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_26 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Calibration corrections are computed after sufficient samples
- High calibration error increases exploration (higher alpha)
- `roko learn all` shows calibration metrics
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_26 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
