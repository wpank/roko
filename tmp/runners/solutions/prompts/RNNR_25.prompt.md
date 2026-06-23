# RNNR_25: Wire critical path priority into TaskScheduler

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-25`](../ISSUE-TRACKER.md#rnnr-25)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.25
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_25 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: When multiple tasks are ready to dispatch, prioritize critical path
tasks. Currently `next_batch()` returns ready tasks without priority sorting.

## Exact Changes

1. Add `priority: TaskPriority` to `SchedulableTask`:
   ```rust
   pub struct TaskPriority {
       pub critical_path: bool,
       pub fan_out: usize,       // number of downstream dependents
       pub tier: u8,             // 0=mechanical, 1=focused, 2=integrative
   }
   ```
2. In `next_batch()`, sort ready tasks by: critical_path desc, fan_out desc, tier asc
3. Critical path tasks dispatched first (cannot afford failure delay)
4. High fan-out tasks next (unblock the most work)
5. Lower-tier tasks before higher-tier (mechanical tasks complete faster)

## Write Scope

- `crates/roko-runtime/src/task_scheduler.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Critical path tasks always dispatched before non-critical
- [ ] Fan-out priority breaks ties among non-critical tasks
- [ ] Priority does not affect correctness (only dispatch order)
- [ ] Dispatch order is deterministic for the same DAG

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_25 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Critical path tasks always dispatched before non-critical
- Fan-out priority breaks ties among non-critical tasks
- Priority does not affect correctness (only dispatch order)
- Dispatch order is deterministic for the same DAG
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_25 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
