# ORCH_03: Concurrent Task Dispatch in WorkflowEngine

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-03`](../ISSUE-TRACKER.md#orch-03)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.3
- Priority: **P0**
- Effort: 8 hours
- Depends on: `ORCH_02` (source 2.2)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_03 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`WorkflowEngine::run_with_cancel()` (in `crates/roko-runtime/src/workflow_engine.rs`, starting at line ~143) currently runs a serial state machine loop: `step() -> execute action -> feed result back -> step() -> ...`. This works for single-prompt workflows (express/standard/full) but not for multi-task plans.

The `TaskScheduler` (at `crates/roko-runtime/src/task_scheduler.rs`) already provides `next_batch()` which returns multiple ready tasks respecting `max_parallel` and file-exclusion constraints. But there is no code in WorkflowEngine that calls `next_batch()` and dispatches tasks concurrently.

The WorkflowEngine needs a new run mode for multi-task plans that:
1. Calls `TaskScheduler::next_batch()` to get dispatchable tasks
2. Spawns agents in parallel via `tokio::task::JoinSet`
3. Collects results as they complete
4. Updates `TaskScheduler` status (completed/failed)
5. Loops until `TaskScheduler::is_done()`

## Exact Changes

1. Add `max_parallel_tasks: usize` field to `WorkflowRunConfig` (default: 1 for backward compat).
2. Add a `run_plan()` method to `WorkflowEngine` that accepts a `Vec<SchedulableTask>` + `WorkflowRunConfig`.
3. In `run_plan()`, create a `TaskScheduler` with the tasks and `max_parallel_tasks`.
4. Main loop:
   ```rust
   let mut join_set = JoinSet::new();
   loop {
       if scheduler.is_done() { break; }
       if cancel.is_cancelled() { break; }
       let batch = scheduler.next_batch();
       for task_id in batch {
           scheduler.mark_running(task_id);
           let driver = /* clone/arc driver */;
           let worktree_path = /* allocate from WorktreeManager if available */;
           join_set.spawn(async move {
               (task_id, driver.spawn_agent_in_worktree(task_id, ..., worktree_path).await)
           });
       }
       // Wait for at least one to complete
       if let Some(result) = join_set.join_next().await {
           let (task_id, input) = result??;
           match input {
               PipelineInput::AgentCompleted { .. } => {
                   // Run gates, handle merge, mark completed
                   scheduler.mark_completed(&task_id);
               }
               PipelineInput::AgentFailed { error } => {
                   scheduler.mark_failed(&task_id, error);
               }
           }
       }
   }
   ```
5. When `max_parallel_tasks == 1`, the behavior degenerates to serial execution (same as today).

## Design Guidance

Use `tokio::task::JoinSet` for the concurrent dispatch, not manual `tokio::spawn` with a `Vec<JoinHandle>`. JoinSet provides ordered completion and cancellation. The EffectDriver must be `Arc`-wrapped for concurrent use -- its `services` field already uses `Arc` for all trait objects, but `feedback_totals` uses `tokio::sync::Mutex` which is safe for concurrent access.

## Write Scope

- `crates/roko-runtime/src/workflow_engine.rs`
- `crates/roko-runtime/src/pipeline_state.rs`

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

- [ ] New `run_plan()` method dispatches tasks from `TaskScheduler::next_batch()` concurrently
- [ ] Serial execution (max_parallel=1) produces identical results to current behavior
- [ ] Integration test: 3 independent tasks complete in parallel (elapsed < 3x single-task time)
- [ ] File-exclusion constraint: tasks with overlapping files are serialized

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_03 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- New `run_plan()` method dispatches tasks from `TaskScheduler::next_batch()` concurrently
- Serial execution (max_parallel=1) produces identical results to current behavior
- Integration test: 3 independent tasks complete in parallel (elapsed < 3x single-task time)
- File-exclusion constraint: tasks with overlapping files are serialized
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_03 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
