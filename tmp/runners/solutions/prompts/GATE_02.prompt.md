# GATE_02: Add feedback, failure_classification, and spc_alerts to GateReport

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-02`](../ISSUE-TRACKER.md#gate-02)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.2
- Priority: **P0**
- Effort: 1 hour
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_02 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`GateReport` at `crates/roko-core/src/foundation.rs:300` has only `verdicts: Vec<GateVerdict>`. Callers that need feedback must call `feedback_for_agent()` separately (only orchestrate.rs does). By including feedback and failure classification in the report, all callers get structured feedback for free.

`GateFeedback` from `crates/roko-gate/src/feedback.rs:53` has fields: `rung`, `passed`, `errors`, `warnings`, `suggestions`. `GateFailureClassification` from `crates/roko-gate/src/compile_errors.rs:180` has fields: `gate`, `primary`, `failure_kind`, `retry_policy`, `summary`, `classes`, `recommended_action`, `cargo_fix_candidate`.

Since roko-core cannot depend on roko-gate, these fields must use serialized representations or a core-level type.

## Exact Changes

1. Add a new struct to `crates/roko-core/src/foundation.rs`:
   ```rust
   /// Structured feedback extracted from gate output. Gate-layer detail types
   /// serialize into these core-level types so callers don't need roko-gate.
   #[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
   pub struct GateReportFeedback {
       /// Error-level items (must fix).
       #[serde(default)]
       pub errors: Vec<String>,
       /// Warning-level items (should fix).
       #[serde(default)]
       pub warnings: Vec<String>,
       /// Actionable suggestions.
       #[serde(default)]
       pub suggestions: Vec<String>,
   }

   /// Coarse classification of a gate failure for retry/replan decisions.
   #[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
   pub struct GateReportClassification {
       /// Primary failure class (e.g., "SyntaxError", "ImportError").
       #[serde(default)]
       pub primary_class: String,
       /// Recommended action: "retry", "replan", "blocked", "needs_human".
       #[serde(default)]
       pub recommended_action: String,
       /// Concise failure summary.
       #[serde(default)]
       pub summary: String,
       /// Whether cargo fix could resolve the issue.
       #[serde(default)]
       pub cargo_fix_candidate: bool,
   }

   /// Statistical process control alert from the adaptive threshold system.
   #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
   pub struct GateReportSpcAlert {
       /// Rung number where the alert fired.
       pub rung: u32,
       /// Alert kind: "cusum_shift", "ewma_out_of_control", "ewma_warning", "change_point".
       pub kind: String,
       /// Alert detail (e.g., EWMA value, change probability).
       #[serde(default)]
       pub detail: String,
   }
   ```
2. Add fields to `GateReport`:
   ```rust
   pub struct GateReport {
       pub verdicts: Vec<GateVerdict>,
       /// Structured feedback from the first failing gate. None if all passed.
       #[serde(default, skip_serializing_if = "Option::is_none")]
       pub feedback: Option<GateReportFeedback>,
       /// Failure classification for retry/replan routing. None if all passed.
       #[serde(default, skip_serializing_if = "Option::is_none")]
       pub failure_classification: Option<GateReportClassification>,
       /// SPC alerts drained after this pipeline run.
       #[serde(default, skip_serializing_if = "Vec::is_empty")]
       pub spc_alerts: Vec<GateReportSpcAlert>,
       /// Whether Hotelling's T-squared detected a joint anomaly across gates.
       #[serde(default)]
       pub joint_anomaly: bool,
   }
   ```
3. Derive `serde::Serialize, serde::Deserialize` on `GateReport` (currently only `Debug, Clone`).
4. Update `GateReport::all_passed()` -- logic unchanged (already checks `passed && !skipped`).
5. Update all sites constructing `GateReport { verdicts }` to include the new fields as defaults. Known sites:
   - `crates/roko-gate/src/gate_service.rs:365` in `run_gates()`
   - `crates/roko-runtime/src/effect_driver.rs` and `workflow_engine.rs` test mocks
   - `crates/roko-cli/src/run.rs` test mocks

## Design Guidance

Use `String`-typed fields in the core-level structs rather than importing roko-gate enums. GateService will convert from `GateFeedback -> GateReportFeedback` and `GateFailureClassification -> GateReportClassification` using simple `From` impls in roko-gate. This keeps roko-core free of roko-gate dependency.

## Write Scope

- `crates/roko-core/src/foundation.rs`

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

- [ ] `GateReport` has `feedback`, `failure_classification`, `spc_alerts`, `joint_anomaly` fields
- [ ] All construction sites updated
- [ ] Serde derives present on `GateReport`

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_02 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `GateReport` has `feedback`, `failure_classification`, `spc_alerts`, `joint_anomaly` fields
- All construction sites updated
- Serde derives present on `GateReport`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_02 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
