# ORCH_16: Wire Speculative Task Dispatch

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-16`](../ISSUE-TRACKER.md#orch-16)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.16
- Priority: **P2**
- Effort: 6 hours
- Depends on: `ORCH_03` (source 2.3)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_16 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`SpeculativeExecution` struct exists at `crates/roko-orchestrator/src/executor/mod.rs:65-75`:
```rust
pub struct SpeculativeExecution {
    pub plan_id: String,
    pub task: String,
    pub expected_minutes: u32,
    pub elapsed_minutes: u32,
    pub backup_role: AgentRole,
}
```

The `ExecutorConfig` has `speculative_threshold_multiplier` (line 171). But no code in Runner v2 or WorkflowEngine triggers speculative spawns.

The trigger condition (from the PLAN): speculatively dispatch a task when it is on the critical path, its dependencies are 80%+ complete, and speculative cost is within budget.

## Exact Changes

1. Add a `speculative_candidates()` method to `TaskScheduler` that returns tasks where:
   - The task is `Blocked` (not yet ready)
   - 80%+ of its dependencies are `Completed`
   - The remaining dependencies are `Running` (likely to complete soon)
2. In the `run_plan()` loop, after dispatching the normal batch, check for speculative candidates.
3. Spawn speculative agents in separate worktrees with a `CancelToken`.
4. When a dependency fails, cancel the speculative agent via `CancelToken`.
5. When all dependencies complete and the task becomes ready, "adopt" the speculative execution -- do not re-dispatch.
6. Track speculative outcomes (hit/miss) for cost accounting.

## Design Guidance

Keep speculative execution behind a config flag (`enable_speculation: bool`, default false). Track speculative cost separately from normal cost. Do not speculate on more than `max_parallel / 2` tasks simultaneously to prevent resource exhaustion.

## Write Scope

- `crates/roko-runtime/src/workflow_engine.rs`
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

- [ ] Speculative dispatch triggers when dependencies are 80%+ complete
- [ ] Speculative agent is cancelled when a dependency fails
- [ ] Speculative agent result is adopted when all dependencies complete
- [ ] Speculation disabled by default (opt-in)
- [ ] Cost tracking distinguishes speculative vs normal execution

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_16 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Speculative dispatch triggers when dependencies are 80%+ complete
- Speculative agent is cancelled when a dependency fails
- Speculative agent result is adopted when all dependencies complete
- Speculation disabled by default (opt-in)
- Cost tracking distinguishes speculative vs normal execution
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_16 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
