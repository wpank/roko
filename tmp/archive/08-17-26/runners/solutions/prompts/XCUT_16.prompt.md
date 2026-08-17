# XCUT_16: Wire GracefulShutdown into roko daemon

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-16`](../ISSUE-TRACKER.md#xcut-16)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.16
- Priority: **P8**
- Effort: 3 hours
- Depends on: `XCUT_14` (source 19.14), `XCUT_15` (source 19.15)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_16 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`roko daemon stop` sends SIGTERM but does not coordinate with the daemon's internal subsystems. The daemon may be mid-plan-run when killed, leaving worktrees in dirty state, agent processes orphaned, and learning files unflushed. `crates/roko-cli/src/daemon.rs` references `force_shutdown` (one of the 9 files matching that pattern).

## Exact Changes

1. In daemon main loop, create `GracefulShutdown::with_deadline(Duration::from_secs(10))`.
2. Register hooks:
   - `"plan-runner"`: signal CancelToken, wait for current task to checkpoint.
   - `"agent-processes"`: call `ProcessSupervisor::shutdown_all()` with 5-second timeout.
   - `"learning-flush"`: flush episode logger, cascade router, efficiency writer.
   - `"worktree-cleanup"`: ensure all worktrees have their latest changes committed.
3. Wire SIGTERM handler to `shutdown.drain().await`.
4. Add `roko daemon stop --timeout <seconds>` flag to override the default deadline.
5. After drain, log a summary of what was flushed and what timed out.

## Write Scope

- `crates/roko-cli/src/daemon.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `roko daemon stop` drains all subsystems before exiting
- [ ] Agent processes are killed if they do not exit within 5 seconds
- [ ] Learning files are flushed to disk before exit
- [ ] `roko daemon stop --timeout 2` uses a 2-second deadline

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_16 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko daemon stop` drains all subsystems before exiting
- Agent processes are killed if they do not exit within 5 seconds
- Learning files are flushed to disk before exit
- `roko daemon stop --timeout 2` uses a 2-second deadline
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_16 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
