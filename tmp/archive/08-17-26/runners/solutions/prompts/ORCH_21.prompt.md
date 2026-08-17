# ORCH_21: Wave-Boundary Gate Execution

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-21`](../ISSUE-TRACKER.md#orch-21)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.21
- Priority: **P1**
- Effort: 5 hours
- Depends on: `ORCH_03` (source 2.3), `ORCH_04` (source 2.4)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_21 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Per-task compilation takes 15-40 minutes (cargo check for 18 crates). Wave gating -- running compilation once per wave of tasks instead of per-task -- reduces this to 3-8 minutes per wave. The mega-parity runner used this pattern with PARALLEL=15 and achieved 10x speedup.

Three gating strategies:
1. **Per-task**: Each task runs gates individually (safest, slowest)
2. **Wave**: Gates run once after all tasks in a wave complete and merge (balanced)
3. **Deferred**: No gates during execution; compile at end (fastest, riskiest)

The `UnifiedTaskDag::waves()` method in `crates/roko-orchestrator/src/dag.rs` already partitions tasks into waves via BFS layering.

## Exact Changes

1. Add a `gate_strategy` field to `WorkflowConfig`:
   ```rust
   pub enum GateStrategy {
       PerTask,     // default: run gates after each task
       PerWave,     // run gates after each wave completes
       Deferred,    // no gates during execution
   }
   ```
2. In `run_plan()`, track wave membership for each task.
3. When `GateStrategy::PerWave`:
   - After each task completes, merge its worktree into the integration branch
   - When all tasks in the current wave have completed and merged:
     - Run gates on the integration branch (not individual worktrees)
     - If gates fail, identify the offending merge via `git bisect` on merge commits
     - Retry only the offending task(s)
4. When `GateStrategy::Deferred`:
   - Skip all gates during execution
   - After all tasks complete, run a single gate pass on the final state
5. Add a "no-build" prompt section injected when `GateStrategy != PerTask`, telling agents not to compile.

## Design Guidance

Wave gating requires tracking which wave each task belongs to. This can be derived from `TaskScheduler` or computed via `UnifiedTaskDag::waves()`. The offending-merge identification for wave gate failures is best done via git bisect on the merge commits within the wave.

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

- [ ] `GateStrategy::PerWave` runs gates once per wave, not per task
- [ ] `GateStrategy::Deferred` skips all gates until the end
- [ ] Wave gate failure identifies which task caused the regression
- [ ] "No-build" prompt section injected for wave/deferred strategies
- [ ] Per-task gating (default) is unchanged from current behavior

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_21 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `GateStrategy::PerWave` runs gates once per wave, not per task
- `GateStrategy::Deferred` skips all gates until the end
- Wave gate failure identifies which task caused the regression
- "No-build" prompt section injected for wave/deferred strategies
- Per-task gating (default) is unchanged from current behavior
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_21 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
