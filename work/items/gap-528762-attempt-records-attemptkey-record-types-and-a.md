+++
id = "gap-528762"
kind = "gap"
title = "Attempt records: AttemptKey, record types and a telemetry writer (S01.P0-0)"
status = "open"
triage = "verified"
severity = "p1"
goal = "truth"
size = "M"
subsystem = ["roko-learn/telemetry", "roko-learn/routing_log"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e4"
discovered_from = "tmp/cybernetic-harness/specs/S01-instrumentation.md (P0-0, §4.2, §4.7, §5); workstreams/assessment/W3a-crosswalk-core.md (reuse risk 8)"
anchors = ["crates/roko-learn/src/telemetry/mod.rs", "crates/roko-learn/src/routing_log.rs::RoutingDecisionLog"]
lane = "rust-cold"
parent = "spec-b7303f"
links = { depends_on = [], blocks = [], related = ["gap-96f7ed", "gap-8cb382"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn attempt_key_matches_receipt_idempotency_layout' crates/roko-learn/src/ && cargo test -p roko-learn --lib telemetry::"
+++

## Problem

There is no attempt identity and no attempt record:

- `GraphTaskDispatcher::next_attempt_id` builds `"{plan}/{task}/a{n}"` from an in-memory counter that restarts with
  the process, so a resumed run reuses keys.
- `RunMetricsRecord` mints its own `graph-run-<ms>` id (`plan_runner.rs:1595`).
- Episodes, efficiency rows and cost rows cannot be joined per attempt.
- No typed verdict record exists, and no writer.

## Why it matters

This is the first step of S01 Phase 0. The rest of epic spec-b7303f, and every S02–S06 loop, reads these records.
The item is library-only, so it can start now.

## Where

- **New:** `crates/roko-learn/src/telemetry/{mod,records,assign,writer}.rs`.
- **Extend:** `crates/roko-learn/src/routing_log.rs::RoutingDecisionLog`, which already holds candidates, the
  selected model and the outcome.
- **Reuse:**
  - `roko_fs::log_rotation::append_jsonl_line_sync` and `roko_fs::layout` `run_dir`;
  - the `{run}:{plan}:{task}:{attempt}` layout of `TaskAttemptReceiptV1.idempotency_key`
    (`roko-execution/src/feedback/receipt.rs`);
  - `PromptAttemptKey` (`roko-learn/src/prompt_experiment.rs`).

## Current state

Checked at `41c7ffbd6`: roko-learn has no `telemetry` module. Two decision-record types exist:
- `roko-core/src/forensic.rs::RouterDecisionRecord`: the selected route, alternatives and confidence;
- `RoutingDecisionLog`: read only by `cascade_router.rs` and `prediction.rs`.

W3a: extend one of them; do not add a third.

## Plan

1. **`AttemptKey`** with `run_id`, `plan_id`, `task_id` and a 1-based `attempt`, plus a `chain_key` and the
   `attempt_open` line (S01 §4.2, §5.2). The ordinal is durable: it starts from the highest ordinal already in
   `runs/<run_id>/attempts.jsonl`. Add converters to `PromptAttemptKey` and to the receipt key layout.
2. **`VerdictRecord`** holds:
   - the outcome, from `TaskGateVerdict`;
   - `failure_class` and `blame`;
   - `learning_label`: 1, 0 or null;
   - the executed model, the cost and `cost_source`.

   Add the `RunManifest` type too; gap-8cb382 writes it.
3. **Decision records:** add `#[serde(default)]` fields to `RoutingDecisionLog` (`attempt_key`, `source`,
   `default_model`, `propensity`) instead of creating a new type.
4. **`assign()`**, a pure BLAKE3 hash, and **`TelemetryWriter`**, with a bounded channel (4096), a per-run `seq`,
   `record_id` dedupe and a counter of dropped records.

## Done when

- [ ] Every record type round-trips through JSON, and old `RoutingDecisionLog` rows still parse.
- [ ] `attempt_key_matches_receipt_idempotency_layout` passes, and so do tests for `assign()` uniformity (χ² over
      10⁵ keys), dedupe and the drop counter.
- [ ] The `[[verify]]` command passes.

## Notes

- There are no call sites here; gap-96f7ed threads the key through dispatch.
- roko-learn does not depend on roko-execution, so convert to the receipt key through the string layout.
- The only shared line is `pub mod telemetry;` in roko-learn's `lib.rs`.
- Implemented on `work/gap-528762` at `5fdc6c5f2`; cargo verification deferred to the batch check.
- Follows S01 v1.2 (gap-3c430e): `AttemptUsage` has the five disjoint token classes plus `tokens_reasoning`.
  Audit rows' `sha256:` ids are S05's and are not written here, and no decision-point list exists here to rename.
- Type names differ from S01's where the workspace already uses them. The verdict is `AttemptVerdictRecord`
  (`roko_learn::verdict_scorer::VerdictRecord` exists) and the manifest is `RunProvenanceManifest` (roko-runtime
  has a `RunManifest`). The others are `GateVerdictTag` (roko-core `GateVerdict`), `AttemptFailureClass` and
  `VerifyStepVerdict` (roko-gate `FailureClass`, `StepVerdict`), and `ConfigHashProvenance` (roko-core
  `ConfigProvenance`). The wire schemas and field names are S01's. S01's `PredictionRecord` also needs a new name,
  because `roko_learn::prediction::PredictionRecord` exists; this item adds no prediction type.
