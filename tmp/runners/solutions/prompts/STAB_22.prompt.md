# STAB_22: Wire runner v2 CascadeRouter observations

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-22`](../ISSUE-TRACKER.md#stab-22)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.22
- Priority: **P1**
- Effort: 1 hour
- Depends on: `STAB_11` (source 1.11)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_22 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Runner v2 imports CascadeRouter types but never calls `cascade_router.observe()` after task
completion. Grep for `CascadeRouter.*observe` in the runner directory returns zero matches.

## Exact Changes

1. After task completion (success or failure) in `event_loop.rs`, construct a routing observation:
   ```rust
   router.observe(UsageObservation {
       model: task_result.model.clone(),
       role: task_result.role.clone(),
       success: task_result.gate_passed,
       cost: task_result.cost,
       latency_ms: task_result.duration_ms,
   });
   ```
2. Persist router state during the periodic flush (reuse the existing flush interval).

## Design Guidance

The observation should happen synchronously in the event loop -- it is a cheap in-memory
update. Persistence can be batched with other flush operations.

## Write Scope

- `crates/roko-cli/src/runner/event_loop.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `roko plan run` on a 3-task plan produces `observations >= 3` in cascade-router.json

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_22 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko plan run` on a 3-task plan produces `observations >= 3` in cascade-router.json
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_22 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
