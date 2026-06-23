# ORCH_15: Extend WorkflowEngine Checkpoint to Include TaskScheduler State

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-15`](../ISSUE-TRACKER.md#orch-15)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.15
- Priority: **P0**
- Effort: 3 hours
- Depends on: `ORCH_14` (source 2.14)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_15 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`EffectDriver::save_checkpoint()` (at `effect_driver.rs:438-464`) serializes only `PipelineStateV2` to JSON. In the multi-task `run_plan()` flow (from Task 2.3), the `TaskScheduler` state also needs checkpointing. A crash at task 15 of 20 should resume from task 15, not restart from task 1.

## Exact Changes

1. Create a `WorkflowCheckpoint` struct that combines both:
   ```rust
   #[derive(Serialize, Deserialize)]
   pub struct WorkflowCheckpoint {
       pub pipeline_state: PipelineStateV2,
       pub task_scheduler: Option<TaskSchedulerSnapshot>,
       pub cumulative_context: Option<CumulativeContext>,
       pub timestamp_ms: u64,
   }
   ```
2. Add a `save_workflow_checkpoint()` method to EffectDriver that serializes `WorkflowCheckpoint`.
3. Add a `load_workflow_checkpoint()` function that deserializes and returns the components.
4. In `run_plan()`, call `save_workflow_checkpoint()` after each task completion.
5. Add a `resume_plan()` method to WorkflowEngine that loads the checkpoint and reconstructs the TaskScheduler.

## Design Guidance

Use atomic write (tmp + rename) pattern already established in `save_checkpoint()`. The checkpoint file should be at `.roko/state/workflow-{run_id}.json`. Backward compat: if the checkpoint is a bare `PipelineStateV2` (old format), load it as pipeline_state with task_scheduler=None.

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

- [ ] `save_workflow_checkpoint()` writes combined state atomically
- [ ] `load_workflow_checkpoint()` restores TaskScheduler to the correct state
- [ ] Resume after simulated crash skips completed tasks
- [ ] Backward compat: old PipelineStateV2-only checkpoints still loadable

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_15 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `save_workflow_checkpoint()` writes combined state atomically
- `load_workflow_checkpoint()` restores TaskScheduler to the correct state
- Resume after simulated crash skips completed tasks
- Backward compat: old PipelineStateV2-only checkpoints still loadable
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_15 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
