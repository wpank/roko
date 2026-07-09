# GATE_21: Define GateEvent enum

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-21`](../ISSUE-TRACKER.md#gate-21)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.21
- Priority: **P2**
- Effort: 1 hour
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_21 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`RuntimeEvent` at `crates/roko-core/src/runtime_event.rs:56` already has `GateStarted`, `GatePassed`, `GateFailed` variants with `run_id`, `gate_name`, `rung`, `duration_ms`, `output`. These exist but are only emitted from the WorkflowEngine's EffectDriver path. GateService does not emit them.

Rather than adding new event types, extend the existing `RuntimeEvent` variants with additional fields for SPC alerts and threshold updates, or add new variants for pipeline-level events.

## Exact Changes

1. Add new variants to `RuntimeEvent`:
   ```rust
   GateSkipped {
       run_id: String,
       gate_name: String,
       reason: String,
   },
   GatePipelineCompleted {
       run_id: String,
       passed: bool,
       duration_ms: u64,
       gates_run: usize,
       gates_skipped: usize,
       joint_anomaly: bool,
   },
   GateSpcAlert {
       run_id: String,
       rung: u8,
       alert_kind: String,
       detail: String,
   },
   GateThresholdUpdated {
       run_id: String,
       rung: u8,
       old_ema: f64,
       new_ema: f64,
   },
   ```
2. Update `run_id()` and `kind()` match arms for the new variants.
3. Update `Display` impl for new variants.

## Write Scope

- `crates/roko-core/src/runtime_event.rs`

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

- [ ] New RuntimeEvent variants have consistent structure
- [ ] `kind()` returns correct labels for new variants

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_21 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- New RuntimeEvent variants have consistent structure
- `kind()` returns correct labels for new variants
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_21 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
