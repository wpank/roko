# RNNR_01: Wire WorktreeManager into runner event loop for per-task worktrees

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-01`](../ISSUE-TRACKER.md#rnnr-01)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.1
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_01 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: When the runner event loop dispatches a task to an agent, allocate a
worktree via `WorktreeManager::create()` and set the agent's working directory
to the worktree path. Currently all tasks share the main repo working directory
(`RunConfig::workdir`).

## Exact Changes

1. Add `worktree_manager: Option<Arc<WorktreeManager>>` to `RunConfig`
2. Add `worktree_isolation: bool` to `RunConfig` (default `false` for backward compat)
3. In the event loop, when dispatching a task and `worktree_isolation` is true:
   - Call `worktree_manager.create(task_id, branch_name).await?` to get a `WorktreeHandle`
   - Pass `handle.path` as the agent's working directory instead of `config.workdir`
4. After task completes (success or failure), call `worktree_manager.touch(task_id)`
   but do NOT remove the worktree (project rule: never auto-delete worktrees)
5. Store active `WorktreeHandle`s in a `HashMap<String, WorktreeHandle>` on the run
   context for resume support

## Write Scope

- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-cli/src/runner/types.rs`

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

- [ ] `roko plan run` with `worktree_isolation = true` creates worktrees under `.roko/worktrees/`
- [ ] Each task agent operates in its own worktree directory
- [ ] Worktrees survive after task completion (never auto-deleted)
- [ ] `roko plan run` without the flag works exactly as before (no regression)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_01 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko plan run` with `worktree_isolation = true` creates worktrees under `.roko/worktrees/`
- Each task agent operates in its own worktree directory
- Worktrees survive after task completion (never auto-deleted)
- `roko plan run` without the flag works exactly as before (no regression)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_01 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
