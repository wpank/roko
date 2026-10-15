+++
id = "gap-96547f"
kind = "gap"
title = "roko learn self-model fit never reads predictions.jsonl, so nothing measures shadow forecast coverage or the share joined to verdicts"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-cli/learn", "roko-learn/self_model"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-14"
anchors = ["crates/roko-cli/src/commands/learn_self_model.rs::fit_offline", "crates/roko-learn/src/self_model/ingest.rs::read_runs", "crates/roko-learn/src/telemetry/records.rs::AttemptPredictionRecord"]
lane = "rust-cold"
parent = "spec-7a71d6"
links = { depends_on = ["gap-9aca5d"], blocks = [], related = ["gap-b38ed2"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn fit_reports_the_forecast_join_share' crates/roko-cli/ && cargo test -p roko-cli fit_reports_the_forecast_join_share"
+++

## Problem

Nothing measures how many forecasts join an outcome. `roko learn self-model fit` (`fit_offline`,
`crates/roko-cli/src/commands/learn_self_model.rs:230`) reads the S01 runs (`ingest::read_runs`) and the legacy logs,
fits a fresh model and scores it; it never reads the shadow forecasts in `predictions.jsonl` (written per routed
attempt by 6126, `crates/roko-learn/src/telemetry/writer.rs`). No reader of `predictions.jsonl` exists in the
self-model or the fit (A07 §4 surprise 4). So backlog 6129's Done-when 2 ("the share of forecast attempts joined to
verdicts") cannot be met as written, and S04 §7.4's shadow acceptance (≥ 95% of routed attempts forecast, ≥ 99% of
forecasts joined to an outcome) has no measure.

## Why it matters

Goal `cybernetic` (S04 §7.4). NEW-FINDINGS finding 14 (third part). Once shadow is the default (gap-9aca5d), the first
question is whether the logged forecasts are complete and joinable; without the measure, a silent logging gap would
look like a calibrated model. Stage 3 of the epic.

## Where

- `crates/roko-cli/src/commands/learn_self_model.rs::fit_offline` and its `FitReport` (printed by `roko learn
  self-model fit`).
- `crates/roko-learn/src/self_model/ingest.rs::read_runs` (the S01 run records: verdicts per attempt key).
- `crates/roko-learn/src/telemetry/records.rs` (`AttemptPredictionRecord`, identity with `attempt_key`).

## Current state

The prediction rows carry the attempt identity; the verdicts carry the same key. Only the join is missing.

## Plan

1. In `fit_offline` (or a helper in `ingest.rs`), read each run's `predictions.jsonl` and report: routed attempts,
   attempts with a forecast (coverage), forecasts whose attempt has a settled labelled verdict (join share), and the
   unjoined keys' count by reason (no verdict, verdict without a label).
2. Print both shares in `fit`'s report and its `--json`, with `insufficient_n` when there are no predictions.
3. Test `fit_reports_the_forecast_join_share`: a fixture run with 4 routed attempts, 3 predictions and 2 labelled
   verdicts reports coverage 0.75 and join share 2/3.

## Done when

The `[[verify]]` command passes.

## Notes

The live acceptance itself (a real shadow run reaching 95%/99%) needs a live run and is not this item.
