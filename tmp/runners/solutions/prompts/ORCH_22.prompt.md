# ORCH_22: Phase Adapter Between PipelineStateV2 and PlanPhase

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-22`](../ISSUE-TRACKER.md#orch-22)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.22
- Priority: **P2**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_22 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Two state machines model the same concept:
- `PipelineStateV2::Phase` (10 states) at `crates/roko-runtime/src/pipeline_state.rs:365-390`
- `PlanPhase` (14 states) at `crates/roko-core/src/phase.rs`

They are not interoperable. Monitoring tools must handle both. A workflow starting as a simple run (PipelineStateV2) cannot report status in PlanPhase terms.

Differences:
- PipelineStateV2 has `Strategizing, Committing, Cancelled` -- PlanPhase does not
- PlanPhase has `Enriching, Verifying, DocRevision, RegeneratingVerify, Merging, Done, Skipped` -- PipelineStateV2 does not
- PipelineStateV2 `Halted{reason: String}` vs PlanPhase `Failed{reason: FailureKind}`

## Exact Changes

1. Create a `PhaseAdapter` module with bidirectional mapping functions:
   ```rust
   pub fn pipeline_to_plan_phase(phase: &Phase) -> PlanPhase { ... }
   pub fn plan_phase_to_pipeline(phase: &PlanPhase) -> Phase { ... }
   ```
2. Define the mapping:
   - `Pending -> Queued`
   - `Strategizing -> Enriching` (closest semantic match)
   - `Implementing -> Implementing`
   - `Gating -> Gating`
   - `AutoFixing -> AutoFixing`
   - `Reviewing -> Reviewing`
   - `Committing -> Merging`
   - `Complete -> Done`
   - `Halted{reason} -> Failed{FailureKind::Other(reason)}`
   - `Cancelled -> Skipped`
3. Add a common `PhaseLabel` enum that both can map to for unified monitoring.
4. Implement `From<Phase> for PhaseLabel` and `From<PlanPhase> for PhaseLabel`.

## Design Guidance

The adapter should be lossy but not crash -- unmappable states should map to the closest equivalent with a log warning. The `PhaseLabel` enum is the unified monitoring interface; both state machines can report their status through it. Do not attempt to unify the state machines themselves -- that is a much larger change.

## Write Scope

- `crates/roko-runtime/src/pipeline_state.rs`
- `crates/roko-core/src/phase.rs`

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

- [ ] All 10 `Phase` variants map to a `PlanPhase` variant
- [ ] All 14 `PlanPhase` variants map to a `Phase` variant
- [ ] Round-trip: `pipeline_to_plan_phase(plan_phase_to_pipeline(x))` is semantically equivalent to `x`
- [ ] Monitoring code can use `PhaseLabel` for unified status display

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_22 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- All 10 `Phase` variants map to a `PlanPhase` variant
- All 14 `PlanPhase` variants map to a `Phase` variant
- Round-trip: `pipeline_to_plan_phase(plan_phase_to_pipeline(x))` is semantically equivalent to `x`
- Monitoring code can use `PhaseLabel` for unified status display
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_22 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
