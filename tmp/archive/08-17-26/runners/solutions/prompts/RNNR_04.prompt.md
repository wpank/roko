# RNNR_04: Add worktree disk space monitoring

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-04`](../ISSUE-TRACKER.md#rnnr-04)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.4
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_04 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Monitor available disk space before creating worktrees. The mega-parity
runner showed 15 worktrees at 500MB each = 7.5GB, plus cargo builds can add
5-15GB per worktree. Pause dispatch when space is low.

## Exact Changes

1. Add `fn check_disk_space(path: &Path) -> Result<DiskStatus, WorktreeError>`:
   ```rust
   pub struct DiskStatus {
       pub available_bytes: u64,
       pub total_bytes: u64,
   }
   ```
   Use `statvfs` on Unix (via `nix` crate or raw libc)
2. Add `min_disk_bytes: u64` to `WorktreeConfig` (default 5GB = 5_368_709_120)
3. In `create()`, call `check_disk_space()` before creating. Return
   `WorktreeError::InsufficientDisk { available, required }` when below threshold
4. Add a new `InsufficientDisk` variant to `WorktreeError`
5. Emit tracing warning when available space < 2x the estimated worktree size

## Write Scope

_None — this is a documentation/verification-only batch._

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

- [ ] `WorktreeManager::create()` fails gracefully when disk is below threshold
- [ ] Warning logged when disk space is low but not critical
- [ ] Disk check does not block normal operations (< 1ms)
- [ ] New error variant has a clear user-facing message

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_04 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `WorktreeManager::create()` fails gracefully when disk is below threshold
- Warning logged when disk space is low but not critical
- Disk check does not block normal operations (< 1ms)
- New error variant has a clear user-facing message
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_04 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
