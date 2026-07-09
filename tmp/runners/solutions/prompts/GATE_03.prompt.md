# GATE_03: Generate feedback and classification inside GateService

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-03`](../ISSUE-TRACKER.md#gate-03)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.3
- Priority: **P0**
- Effort: 3 hours
- Depends on: `GATE_01` (source 4.1), `GATE_02` (source 4.2)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_03 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`GateService::run_gates()` at `crates/roko-gate/src/gate_service.rs:235` currently returns `Ok(GateReport { verdicts })` with no feedback or classification. The feedback module (`crates/roko-gate/src/feedback.rs:202` `feedback_for_agent()`) and classification module (`crates/roko-gate/src/compile_errors.rs:491` `classify_gate_failure()`) exist and work but are only called from orchestrate.rs.

By moving feedback generation into GateService, all callers get structured feedback automatically. This eliminates AP-7 (feedback as afterthought).

## Exact Changes

1. Add `use crate::feedback::feedback_for_agent;` and `use crate::compile_errors::classify_gate_failure;` to gate_service.rs imports.
2. After the verdict collection loop (line 363), before the `Ok(GateReport { verdicts })` return (line 365), add feedback generation:
   ```rust
   // Generate feedback from first failing gate
   let (feedback, failure_classification) = if let Some(failing) = verdicts.iter().find(|v| !v.passed && !v.skipped) {
       let rung = Self::rung_for_name(&failing.gate_name).unwrap_or(0);
       let fb = feedback_for_agent(&failing.output, rung);
       let core_fb = GateReportFeedback {
           errors: fb.errors,
           warnings: fb.warnings,
           suggestions: fb.suggestions,
       };
       let classification = classify_gate_failure(&failing.gate_name, &failing.output);
       let core_class = GateReportClassification {
           primary_class: format!("{:?}", classification.primary),
           recommended_action: format!("{:?}", classification.recommended_action),
           summary: classification.summary.clone(),
           cargo_fix_candidate: classification.cargo_fix_candidate,
       };
       (Some(core_fb), Some(core_class))
   } else {
       (None, None)
   };
   ```
3. Return the enriched report:
   ```rust
   Ok(GateReport {
       verdicts,
       feedback,
       failure_classification,
       spc_alerts: Vec::new(),  // Wired in Task 4.15
       joint_anomaly: false,     // Wired in Task 4.16
   })
   ```
4. Update existing tests in `gate_service.rs` to assert on the new fields where relevant (at minimum: verify `feedback` is `None` when all gates pass, `Some` when a gate fails).

## Design Guidance

Keep feedback generation synchronous and cheap -- it is pure string parsing, no I/O. The `feedback_for_agent()` function filters noise and classifies severity in ~0.1ms. `classify_gate_failure()` parses cargo JSON diagnostics in ~0.5ms. Both are safe to call on every gate run.

## Write Scope

- `crates/roko-gate/src/gate_service.rs`

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

- [ ] GateService returns `feedback: Some(...)` when a gate fails
- [ ] GateService returns `feedback: None` when all gates pass
- [ ] `failure_classification.recommended_action` is populated on failure

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_03 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- GateService returns `feedback: Some(...)` when a gate fails
- GateService returns `feedback: None` when all gates pass
- `failure_classification.recommended_action` is populated on failure
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_03 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
