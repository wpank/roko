# RNNR_29: Enhance `--resume-plan` with parallel re-execution

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-29`](../ISSUE-TRACKER.md#rnnr-29)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.29
- Priority: **??**
- Effort: ?
- Depends on: `RNNR_20` (source 14.20), `RNNR_21` (source 14.21), `RNNR_22` (source 14.22)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_29 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: When running with `--resume-plan` after a partial failure, re-execute
only failed/blocked tasks while preserving successful results. Support parallel
re-execution of independent failed tasks.

## Exact Changes

1. On resume, load result files (14.20) and task fingerprints (14.21)
2. Classify tasks: `success` -> skip, `failed` -> re-execute, `blocked` -> check
   if dependency now resolved, `in_progress` -> treat as failed (stale)
3. Rebuild the DAG with only tasks that need re-execution
4. Dispatch re-execution tasks in parallel where dependencies allow
5. Merge re-executed tasks via merge queue
6. Support multiple `--resume-plan` cycles (each picks up where last left off)
7. Clear stale worktree locks before re-execution (via `clear_stale_locks()`)

## Write Scope

- `crates/roko-cli/src/runner/event_loop.rs`

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

- [ ] `--resume-plan` skips successful tasks and re-runs failed ones
- [ ] Previously blocked tasks re-evaluated against current dependency state
- [ ] Parallel re-execution works for independent failed tasks
- [ ] Multiple consecutive resume cycles converge to all-success

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_29 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `--resume-plan` skips successful tasks and re-runs failed ones
- Previously blocked tasks re-evaluated against current dependency state
- Parallel re-execution works for independent failed tasks
- Multiple consecutive resume cycles converge to all-success
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_29 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
