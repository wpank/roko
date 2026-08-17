# GATE_18: Act on PRM signals (early termination, model switch)

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-18`](../ISSUE-TRACKER.md#gate-18)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.18
- Priority: **P2**
- Effort: 3 hours
- Depends on: `GATE_17` (source 4.17)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_18 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

With PRM tracking per-task trajectories, the runner can make informed decisions about whether to continue, abandon, or change strategy.

## Exact Changes

1. Define threshold constants:
   ```rust
   const PRM_ABANDON_THRESHOLD: f64 = 0.15;
   const PRM_STALL_THRESHOLD: f64 = -0.05;
   const PRM_STALL_MIN_TURNS: usize = 3;
   ```
2. After computing PRM signals, act on them:
   ```rust
   if promise < PRM_ABANDON_THRESHOLD && prm.history.len() >= 3 {
       tracing::warn!(task_id, promise, "PRM: low promise, abandoning task");
       // Mark task as failed with reason "PRM: low promise"
   } else if progress < PRM_STALL_THRESHOLD && prm.history.len() >= PRM_STALL_MIN_TURNS {
       tracing::warn!(task_id, progress, "PRM: stalled progress, consider model switch");
       // Emit RunnerEvent::ModelSwitchSuggested
   }
   ```
3. Log PRM signals to efficiency events for post-run analysis.

## Write Scope

- `crates/roko-cli/src/runner/event_loop.rs`

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

- [ ] Tasks with consistently declining promise are abandoned early
- [ ] Stalled tasks emit model switch suggestions
- [ ] PRM signals appear in efficiency events

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_18 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Tasks with consistently declining promise are abandoned early
- Stalled tasks emit model switch suggestions
- PRM signals appear in efficiency events
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_18 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
