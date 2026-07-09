# RNNR_06: Add WaveGating phase to PipelineStateV2

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-06`](../ISSUE-TRACKER.md#rnnr-06)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.6
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_06 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Extend the pipeline state machine with a `WaveGating` phase that
accumulates completed tasks and triggers gates at wave boundaries. Currently
`Phase` gates every task individually.

## Exact Changes

1. Add `WaveGating` variant to the `Phase` enum (after `Gating`)
2. Add `wave_gate_mode: WaveGateMode` to `WorkflowConfig`:
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
   pub enum WaveGateMode {
       #[default]
       PerTask,       // current behavior
       PerWave,       // gate after each wave completes
       Deferred,      // gate only at end of plan
   }
   ```
3. When `wave_gate_mode` is `PerWave`, the state machine transitions from
   `Implementing` to `WaveGating` only when all tasks in the current wave
   have reached `AgentCompleted`
4. In `WaveGating`, emit `PipelineOutput::RunGates` once for the entire wave
5. On gate success, transition to dispatching the next wave
6. On gate failure, include which wave failed and the gate output
7. `PerTask` produces identical behavior to current (no regression)

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

- [ ] `WaveGateMode::PerTask` produces identical behavior to current
- [ ] `WaveGateMode::PerWave` runs gates once per wave, not per task
- [ ] State machine transitions covered by unit tests
- [ ] `WaveGateMode::Deferred` gates only at end of plan

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_06 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `WaveGateMode::PerTask` produces identical behavior to current
- `WaveGateMode::PerWave` runs gates once per wave, not per task
- State machine transitions covered by unit tests
- `WaveGateMode::Deferred` gates only at end of plan
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_06 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
