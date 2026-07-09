# ORCH_25: Disk Space Monitoring (ORCH-020)

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-25`](../ISSUE-TRACKER.md#orch-25)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.25
- Priority: **P2**
- Effort: 2 hours
- Depends on: `ORCH_20` (source 2.20)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_25 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The mega-parity runner learned that disk exhaustion from cargo build caches causes silent failures. With parallel execution and worktrees, each worktree can consume 500MB (source) + 5-15GB (target dir). 15 concurrent worktrees = 7.5GB source + potentially 75GB+ target.

WorkflowEngine has no disk space monitoring. Cargo builds in worktrees can exhaust available space without warning.

## Exact Changes

1. Add a `check_disk_space()` utility function:
   ```rust
   async fn check_disk_space(path: &Path) -> Option<u64> {
       #[cfg(unix)]
       {
           use std::os::unix::fs::MetadataExt;
           let stat = nix::sys::statvfs::statvfs(path).ok()?;
           Some(stat.blocks_available() * stat.block_size())
       }
   }
   ```
2. In `run_plan()`, check disk space every 60 seconds (or before each wave).
3. If available space < 5GB, pause dispatch (do not start new tasks, let running ones complete).
4. Log a warning when space < 10GB.
5. Emit `RuntimeEvent::ResourceWarning` when disk is low.

## Design Guidance

The 5GB threshold is a configurable minimum. For development machines, 5GB is reasonable; for servers, it should be higher. Shared target directories (`CARGO_TARGET_DIR`) reduce the per-worktree disk impact from 5-15GB to near zero.

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

- [ ] Disk space check runs periodically during plan execution
- [ ] Dispatch pauses when available space < 5GB
- [ ] Warning logged when space < 10GB
- [ ] RuntimeEvent emitted for monitoring

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_25 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Disk space check runs periodically during plan execution
- Dispatch pauses when available space < 5GB
- Warning logged when space < 10GB
- RuntimeEvent emitted for monitoring
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_25 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
