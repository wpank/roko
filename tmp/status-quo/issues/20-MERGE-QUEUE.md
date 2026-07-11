# Merge Queue Issues

## Critical

### No rollback after failed post-merge regression gate
- `merge.rs:459-485`, `event_loop.rs:2841-2857`: After `git merge --no-ff` succeeds but regression gate fails, merged commit stays permanently. No `git reset` or `git revert`. Subsequent plans' gates fail against corrupted HEAD.
- `PostMergeRunner` has `mark_reverted` and `should_revert` — but never called from merge path.

### Interrupted merge leaves permanent deadlock
- `merge_queue.rs:597-603`, `event_loop.rs:3986-4195`: If process dies after `submit()` stores `Merging` but before `mark_complete`, file locks are re-held on resume but no `spawn_regression` is re-launched. Plan stays in `Merging` forever, blocking all plans sharing those files.

### `Merging` entries not re-dispatched on resume
- `merge_queue.rs:582-610`: `from_snapshot` restores entries with locks but no post-resume step iterates `in_progress_requests()` to re-spawn regression gates.

## High

### `enqueue` replaces `Merging` entry — lock leak
- `merge_queue.rs:242-253`: Overwrites entry status back to `Queued` without releasing file locks from prior `Merging` state. Locks orphaned permanently.

### Empty `files_changed` disables conflict detection
- `event_loop.rs:5582-5592`: If no `files` events were observed, `Vec::new()` means no conflicts detected. Two plans touching same files merge concurrently.

### Channel close drops completion silently
- `merge.rs:507-510`: If `gate_tx` closed during shutdown, completion is dropped. Queue entry stays `Merging` forever.

## Medium

### All merge requests use `priority: 0`
- `event_loop.rs:5591`: Priority mechanism exists but never driven. Ordering is lexicographic plan_id.

### Wrong timeout for regression gate
- `event_loop.rs:1451,5595`: Uses `gate_timeout(config, rung=0)` (compile timeout, ~30-60s). `cargo check --workspace` on large workspace can need several minutes → infinite retries.

### `drain_next` may merge against wrong workdir
- `event_loop.rs:2873-2880`: Uses `ctx.config.workdir` (main repo root) but worktree-per-plan mode has isolated worktrees.
