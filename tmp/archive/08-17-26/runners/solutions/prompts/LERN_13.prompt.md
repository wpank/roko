# LERN_13: Wire Regression Alerting to `roko run` and `roko status`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-13`](../ISSUE-TRACKER.md#lern-13)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.13
- Priority: **P2**
- Effort: 3 hours
- Depends on: `LERN_05` (source 7.5)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_13 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`LearningRuntime::record_completed_run()` returns `LearningUpdate` (at `runtime_feedback.rs:345`). `LearningUpdate` has a `regression_report: Option<RegressionReport>` field. `RegressionReport` (at `regression.rs:93`) has `regressions()`, `warnings()`, `improvements()` methods returning `Vec<&RegressionAlert>`. Each `RegressionAlert` has `metric_name`, `baseline`, `current`, `severity`.

Currently the `LearningUpdate` return value from `record_completed_run()` (at `run.rs:2680`) is discarded with `map_err()`.

## Exact Changes

1. In `run.rs`, capture the `LearningUpdate` from `record_completed_run()`.
2. If `update.regression_report` contains alerts with `severity >= Alert`:
   - Log each at WARN: `"Regression detected: {metric} dropped from {baseline:.2} to {current:.2}"`
3. In `commands/status.rs`, add a "Regressions" section that loads recent `RegressionReport` from the learning state.
4. In `commands/learn.rs`, add regression summary to `roko learn all` output.
5. Save regression report to a file (e.g., `.roko/learn/regressions.json`) for dashboard consumption.

## Write Scope

- `crates/roko-cli/src/run.rs`
- `crates/roko-cli/src/commands/status.rs`
- `crates/roko-cli/src/commands/learn.rs`

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

- [ ] Create 10 passing tasks then 5 failing tasks, verify regression alert in logs
- [ ] `roko status` shows regression section when regressions exist
- [ ] `roko learn all` includes regression summary

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_13 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Create 10 passing tasks then 5 failing tasks, verify regression alert in logs
- `roko status` shows regression section when regressions exist
- `roko learn all` includes regression summary
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_13 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
