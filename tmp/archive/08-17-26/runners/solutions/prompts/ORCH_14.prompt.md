# ORCH_14: Add Serialize/Deserialize to TaskStatus

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-14`](../ISSUE-TRACKER.md#orch-14)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.14
- Priority: **P0**
- Effort: 2 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_14 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`TaskStatus` at `crates/roko-runtime/src/task_scheduler.rs:23-38` is:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Blocked,
    Ready,
    Running,
    Completed,
    Failed { error: String },
    Skipped,
}
```

It lacks `Serialize` and `Deserialize` derives. WorkflowEngine checkpoints `PipelineStateV2` state but not `TaskScheduler` state. A crash during multi-task execution loses all task-level progress.

## Exact Changes

1. Add `Serialize, Deserialize` derives to `TaskStatus` enum.
2. Add `Serialize, Deserialize` derives to `SchedulableTask` struct.
3. Add a `TaskSchedulerSnapshot` struct:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct TaskSchedulerSnapshot {
       pub task_statuses: HashMap<String, TaskStatus>,
       pub max_parallel: usize,
   }
   ```
4. Add `checkpoint()` -> `TaskSchedulerSnapshot` and `from_snapshot()` -> `TaskScheduler` methods.
5. In `from_snapshot()`, reconstruct the `tasks` map from the original task definitions and apply the saved statuses.

## Design Guidance

The snapshot should store task statuses only, not the full task definitions (those come from the plan file). On resume, the caller provides the task definitions and the snapshot provides the statuses. Tasks not in the snapshot default to `Blocked` (new tasks added between runs). Tasks in the snapshot but not in the task list are ignored (deleted tasks).

## Write Scope

- `crates/roko-runtime/src/task_scheduler.rs`

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

- [ ] `TaskStatus` serializes and deserializes correctly
- [ ] `checkpoint()` captures current task statuses and max_parallel
- [ ] `from_snapshot()` restores a TaskScheduler to the saved state
- [ ] Completed tasks remain completed after resume
- [ ] Running tasks revert to `Ready` on resume (conservative -- they may have crashed)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_14 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `TaskStatus` serializes and deserializes correctly
- `checkpoint()` captures current task statuses and max_parallel
- `from_snapshot()` restores a TaskScheduler to the saved state
- Completed tasks remain completed after resume
- Running tasks revert to `Ready` on resume (conservative -- they may have crashed)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_14 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
