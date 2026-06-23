# ORCH_04: Post-Task Merge via MergeQueue

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-04`](../ISSUE-TRACKER.md#orch-04)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.4
- Priority: **P0**
- Effort: 6 hours
- Depends on: `ORCH_03` (source 2.3)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_04 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

After a task completes in its worktree, its changes need to be merged into the integration branch. The EffectDriver currently commits directly via `git add -A && git commit` in `commit()` (at `effect_driver.rs:340-421`) with no merge queue coordination.

`MergeQueue` at `crates/roko-orchestrator/src/merge_queue.rs` provides file-conflict-aware serialized merging. `PostMergeRunner` at `crates/roko-orchestrator/src/post_merge.rs` runs regression gates after each merge. Runner v2 uses both via `PlanMerger` at `crates/roko-cli/src/runner/merge.rs`.

The merge flow should be:
1. Task completes in worktree -> run gates in worktree
2. If gates pass -> `MergeQueue::enqueue()` with files_changed
3. Wait for MergeQueue slot (file-overlap check)
4. Merge worktree branch into integration branch
5. Run `PostMergeRunner::check()` on integration branch
6. If post-merge passes -> `MergeQueue::complete()` -> mark task completed
7. If post-merge fails -> `MergeQueue::fail()` -> revert merge -> retry task

## Exact Changes

1. Add a `merge_task_result()` method to EffectDriver that:
   - Gets the list of changed files via `git diff --name-only` in the worktree
   - Creates a `MergeRequest` with plan_id=task_id, branch_name from worktree handle, files_changed
   - Calls `merge_queue.enqueue(request)` if merge_queue is available
   - Polls `merge_queue.dequeue()` to get the merge slot
   - Executes `git merge` from worktree branch into integration branch
   - Calls `merge_queue.complete()` on success or `merge_queue.fail()` on error
2. Add a `run_post_merge_gate()` method that runs compile/clippy on the integration branch after merge
3. Wire `merge_task_result()` into the `run_plan()` loop after gates pass and before marking completed
4. When `merge_queue` is `None`, fall back to the current `commit()` behavior (direct commit in workdir)

## Design Guidance

The merge step should be a separate method, not inlined into the run loop, so it can be tested independently. The PostMergeRunner pattern from roko-orchestrator should be reused rather than reimplemented. Consider adding a `MergeService` trait to keep EffectDriver decoupled from the git implementation.

## Write Scope

- `crates/roko-runtime/src/workflow_engine.rs`
- `crates/roko-runtime/src/effect_driver.rs`

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

- [ ] Tasks in separate worktrees merge their changes via MergeQueue
- [ ] File-overlapping merges are serialized (not concurrent)
- [ ] Post-merge regression gate catches integration errors
- [ ] Fallback: when `merge_queue` is `None`, commit behavior is unchanged
- [ ] Unit test: two tasks with overlapping files merge sequentially

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_04 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Tasks in separate worktrees merge their changes via MergeQueue
- File-overlapping merges are serialized (not concurrent)
- Post-merge regression gate catches integration errors
- Fallback: when `merge_queue` is `None`, commit behavior is unchanged
- Unit test: two tasks with overlapping files merge sequentially
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_04 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
