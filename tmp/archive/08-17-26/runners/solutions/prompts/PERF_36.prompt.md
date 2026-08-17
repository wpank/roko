# PERF_36: Multi-Run Consistency Mode for Bench Harness

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-36`](../ISSUE-TRACKER.md#perf-36)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.36
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_36 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Add `trials: usize` to benchmark options. When >1, run each task K
times and compute consistency metrics (pass rate, K-trial consistency, token
variance).

## Exact Changes

1. Add `pub trials: usize` to `SweBenchOptions`, default 1
2. When `trials > 1`:
   - Run each task K times with different seeds
   - Collect pass/fail per trial
   - Compute: pass rate, K-trial consistency, token usage CoV
3. Include metrics in result output
4. `trials = 1`: unchanged behavior

## Write Scope

- `crates/roko-cli/src/bench.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/10-PERFORMANCE.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `trials = 1`: identical behavior
- [ ] `trials = 5`: each task runs 5x, results include per-trial outcomes
- [ ] A task passing 3/5 gets 60% pass rate

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_36 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `trials = 1`: identical behavior
- `trials = 5`: each task runs 5x, results include per-trial outcomes
- A task passing 3/5 gets 60% pass rate
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_36 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
