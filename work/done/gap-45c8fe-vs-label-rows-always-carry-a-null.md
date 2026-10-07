+++
id = "gap-45c8fe"
kind = "gap"
title = "vs.label rows always carry a null prediction_id, so the audit can't see M3's own forecast per attempt"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-cli/audit", "roko-learn/telemetry"]
created = 2026-10-03
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "c71eabdb0"
source = "wave-9 follow-up reports 2026-10-03 (PK60 gap-940e44)"
discovered_from = "gap-940e44"
anchors = ["crates/roko-cli/src/audit/labels.rs", "crates/roko-learn/src/telemetry/records.rs::AttemptPredictionRecord"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn vs_label_carries_the_attempt_s_prediction_id' crates/roko-cli/ && cargo test -p roko-cli vs_label_carries_the_attempt_s_prediction_id"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T06:53:23Z"
commit = "c71eabdb0"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-04T04:52:59Z"
forced = false
evidence = "Gate 14b (merged c71eabdb0): verify vs_label_carries_the_attempt_s_prediction_id passes. The attempt's prediction id is noted with the run's lottery, carried on AuditUnit.prediction_id (also in the vault queue file) and copied into vs.label rows."
+++

## Problem

`crates/roko-cli/src/audit/labels.rs::write_label` (module doc comment, lines 1-12) already names the gap in its
own words: "`prediction_id` is null: the audit does not see M3's forecast." The code sets
`prediction_id: None` unconditionally (line ~101) on every `vs.label` row it writes.

## Why it matters

Goal: cybernetic, M3/M4 (S04/S05 interface). DP5's late-label hook (6129, `VsLearner`) already uses a `vs.label`
row's known VS outcome to teach the self-model, weighted by 1/π — but with no `prediction_id`, nothing can join
a specific audited attempt's `vs.label` outcome back to the self-model's own forecast for that same attempt.
That join is exactly what calibration diagnostics need: "did M3 predict this attempt would pass, and did the
audit confirm or refute it" — currently unanswerable per-attempt, only in aggregate.

## Where

- `crates/roko-cli/src/audit/labels.rs` (the `vs.label` writer, `prediction_id: None`).
- `crates/roko-learn/src/telemetry/records.rs::AttemptPredictionRecord` (`prediction_id`, `::prediction_id()`,
  lines ~1806-1853 — the id this should actually carry, already computed elsewhere from the attempt key and
  predictor version).

## Current state

Unaddressed; the gap is self-documented in the writer's own module comment but untracked.

## Plan

1. Thread the attempt's `AttemptPredictionRecord::prediction_id` (already computed at prediction time) through
   to wherever `write_label` builds the `vs.label` row, so it's populated instead of always `None`.
2. Add a regression test: a `vs.label` row for an attempt that had an M3 forecast carries that forecast's
   `prediction_id`; one with no forecast still carries `None`.

## Done when

- A `vs.label` row for an attempt with a recorded M3 forecast carries that forecast's `prediction_id`.
- The `[[verify]]` command passes.

## Notes

- Discovered during PK60's work (gap-940e44, done).
- Out of scope per the report: the related "V0 floor" point for DP3 now belongs to PK64 (gap-2e4a81) — not
  duplicated here.
