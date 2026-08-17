# ORCH_20: Adaptive Parallelism Controller

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-20`](../ISSUE-TRACKER.md#orch-20)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.20
- Priority: **P2**
- Effort: 4 hours
- Depends on: `ORCH_03` (source 2.3)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_20 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

With parallel execution enabled, runtime signals should dynamically adjust concurrency:
- Error rate > 30% -> reduce max_parallel by 50%
- Error rate < 10% -> increase max_parallel by 1 (up to configured max)
- Merge conflicts > 20% -> reduce max_parallel
- Disk < 5GB -> pause dispatch entirely

The mega-parity runner validated that PARALLEL=15 was optimal for 20 API workers, but this varies with task complexity and error rates.

## Exact Changes

1. Create an `AdaptiveParallelism` struct:
   ```rust
   pub struct AdaptiveParallelism {
       base_max_parallel: usize,
       current_max_parallel: usize,
       outcome_window: VecDeque<bool>,  // true=success, false=failure
       window_size: usize,              // default: 10
   }
   ```
2. Add `adjust(&mut self, task_succeeded: bool) -> usize` that updates the window and recalculates.
3. Add `check_disk_space()` that queries available space via `statvfs` (unix) or `GetDiskFreeSpaceEx` (windows).
4. Wire into `run_plan()`: before each `next_batch()`, call `adaptive.adjust()` and pass `current_max_parallel` to the scheduler.
5. Add config: `adaptive_parallelism: bool` (default false) in `WorkflowConfig`.

## Design Guidance

The adjustment should be smooth (no oscillation). Use a sliding window of the last N task outcomes. Minimum parallel is always 1. Maximum parallel is the configured `max_parallel_tasks`. Disk space checking should be lazy (check every 60 seconds, not every dispatch).

## Write Scope

- `crates/roko-runtime/src/workflow_engine.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Error rate spike (3/10 failures) reduces max_parallel from 4 to 2
- [ ] Low error rate (0/10 failures) increases max_parallel back toward configured max
- [ ] Disk space below 5GB pauses dispatch
- [ ] Disabled by default; opt-in via config
- [ ] Unit test: simulate error rate changes and verify parallelism adjustments

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_20 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Error rate spike (3/10 failures) reduces max_parallel from 4 to 2
- Low error rate (0/10 failures) increases max_parallel back toward configured max
- Disk space below 5GB pauses dispatch
- Disabled by default; opt-in via config
- Unit test: simulate error rate changes and verify parallelism adjustments
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_20 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
