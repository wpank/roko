# EVAL_31: Pipeline arm bandits (CascadeRouter extension)

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-31`](../ISSUE-TRACKER.md#eval-31)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.31
- Priority: **P2**
- Effort: 5 hours
- Depends on: `EVAL_05` (source 5.5)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_31 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`PipelineArm` represents a pipeline configuration (model + retrieval_k + flags). Feature vector for LinUCB contextual bandit. Integrates with existing `CascadeRouter` at `crates/roko-learn/src/cascade_router.rs`.

## Exact Changes

1. Define `PipelineArm { model: String, retrieval_k: u32, with_clarifying_turn: bool, with_post_fixer: bool }`.
2. Implement `to_features(&self) -> Vec<f64>` for bandit integration.
3. Define `FlywheelEvent { id, timestamp, event_type: FlywheelEventType, trace_id, task_id, metadata }`.
4. Define `FlywheelEventType { TraceEmitted, AutoGradeCompleted, PreferenceMined, PatternExtracted, CurriculumGenerated, ExperimentCreated, CanaryEvaluated, AnchorRotated, DriftDetected }`.

## Write Scope

- `crates/roko-eval/src/lib.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Test feature vector generation
- [ ] Test FlywheelEvent serialization round-trip

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_31 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Test feature vector generation
- Test FlywheelEvent serialization round-trip
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_31 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
