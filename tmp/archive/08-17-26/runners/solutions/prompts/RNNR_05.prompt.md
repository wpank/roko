# RNNR_05: Add worktree cleanup utilities (non-destructive)

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-05`](../ISSUE-TRACKER.md#rnnr-05)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.5
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_05 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Add methods for cleaning up worktree artifacts (build caches, temp
files) without deleting worktrees or branches. Incremental build caches are the
primary disk consumer (~2GB per worktree with cargo builds).

## Exact Changes

1. Add `pub async fn clean_build_cache(&self, task_id: &str) -> Result<u64, WorktreeError>`
   that removes `target/` directories within the worktree; returns bytes freed
2. Add `pub async fn clean_all_build_caches(&self) -> Result<u64, WorktreeError>` for batch cleanup
3. Add `pub fn worktree_sizes(&self) -> Result<Vec<(String, u64)>, WorktreeError>` for monitoring
4. Do NOT add any method that deletes worktrees or branches automatically
   (project rule: never delete worktrees or branches)
5. Extend existing `clear_stale_locks()` (already at line 594) to also handle
   locks in worktree subdirectories

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

- [ ] `clean_build_cache()` removes only `target/` directories, nothing else
- [ ] Worktree source files and git history are never touched
- [ ] `worktree_sizes()` returns accurate sizes for monitoring display
- [ ] No method auto-deletes worktrees or branches

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_05 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `clean_build_cache()` removes only `target/` directories, nothing else
- Worktree source files and git history are never touched
- `worktree_sizes()` returns accurate sizes for monitoring display
- No method auto-deletes worktrees or branches
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_05 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
