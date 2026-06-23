# GATE_17: Instantiate ProcessRewardModel per task in Runner v2

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-17`](../ISSUE-TRACKER.md#gate-17)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.17
- Priority: **P2**
- Effort: 3 hours
- Depends on: `GATE_07` (source 4.7)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_17 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`ProcessRewardModel` at `crates/roko-gate/src/process_reward.rs:51` tracks per-turn `TurnSnapshot` (rung, verdicts, error_count, diff_lines) and derives `promise()` (probability of eventual success) and `progress()` (trajectory delta). It is fully implemented and tested but never instantiated at runtime.

## Exact Changes

1. Add `HashMap<String, ProcessRewardModel>` to `RunState` (per-task PRM):
   ```rust
   pub prm_per_task: HashMap<String, ProcessRewardModel>,
   ```
2. After each gate pipeline run, record a `TurnSnapshot`:
   ```rust
   let snapshot = TurnSnapshot {
       rung: completion.rung,
       verdicts: completion.verdicts.iter().map(|v| /* convert */).collect(),
       error_count: completion.verdicts.iter().filter(|v| !v.passed).count() as u32,
       diff_lines: 0, // Filled from agent output if available
   };
   let prm = run_state.prm_per_task.entry(task_id.clone()).or_insert_with(ProcessRewardModel::new);
   prm.history.push(snapshot);
   ```
3. Compute and log promise/progress:
   ```rust
   let promise = prm.promise();
   let progress = prm.progress();
   tracing::info!(task_id = %task_id, promise, progress, "PRM signals");
   ```

## Write Scope

- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-cli/src/runner/state.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] PRM is created per task and updated on each gate completion
- [ ] Promise and progress values are logged
- [ ] History grows with each attempt

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_17 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- PRM is created per task and updated on each gate completion
- Promise and progress values are logged
- History grows with each attempt
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_17 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
