# RNNR_26: Implement auto-cherry-pick conveyor belt

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-26`](../ISSUE-TRACKER.md#rnnr-26)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.26
- Priority: **??**
- Effort: ?
- Depends on: `RNNR_03` (source 14.3), `RNNR_20` (source 14.20)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_26 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Background process that watches for completed task merges and
cherry-picks them into a target branch. The mega-parity runner's "conveyor
belt" pattern.

## Exact Changes

1. Add `AutoPickConfig` to `PlanMerger`:
   ```rust
   pub struct AutoPickConfig {
       pub target_branch: String,
       pub interval_secs: u64,         // polling interval (default 90)
       pub auto_resolve: bool,         // accept --theirs on conflict
       pub verify_after_pick: bool,    // run cargo check after each cycle
   }
   ```
2. Add `spawn_auto_pick(config, merge_queue) -> JoinHandle` that polls for
   completed merges and cherry-picks to target branch
3. On conflict: if `auto_resolve` is true, use `git checkout --theirs .`;
   otherwise mark as needing manual resolution
4. Save pick state to `.roko/state/auto-pick.json` (survives restart)
5. Track cherry-pick events for monitoring

## Write Scope

- `crates/roko-cli/src/runner/merge.rs`

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

- [ ] Completed task changes auto cherry-picked to target branch
- [ ] Conflict resolution respects `auto_resolve` config
- [ ] Pick state survives process restart
- [ ] Cherry-pick progress visible via events/dashboard

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_26 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Completed task changes auto cherry-picked to target branch
- Conflict resolution respects `auto_resolve` config
- Pick state survives process restart
- Cherry-pick progress visible via events/dashboard
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_26 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
