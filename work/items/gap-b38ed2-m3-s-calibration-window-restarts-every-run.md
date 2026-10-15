+++
id = "gap-b38ed2"
kind = "gap"
title = "M3's calibration window restarts every run and calibration.json is never written, so the self-model can never become eligible or report calibration"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch", "roko-learn/self_model", "roko-cli/graph_execution"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-14"
anchors = ["crates/roko-cli/src/graph_task_dispatch/self_model.rs::SelfModelRuntime", "crates/roko-learn/src/self_model/gate.rs::CalibrationWindow", "crates/roko-learn/src/self_model/gate.rs::write_calibration", "crates/roko-learn/src/self_model/model.rs", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan_body"]
lane = "rust-hot"
parent = "spec-7a71d6"
links = { depends_on = ["gap-9aca5d"], blocks = [], related = ["gap-e464e6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn calibration_window_survives_across_runs' crates/roko-cli/ && cargo test -p roko-cli calibration_window_survives_across_runs"

[[verify]]
command = "grep -rqw 'fn plan_run_writes_calibration_json' crates/roko-cli/ && cargo test -p roko-cli plan_run_writes_calibration_json"
+++

## Problem

Two wiring gaps keep M3's calibration from ever existing outside one run (A07 §4 surprises 1 and 2):

1. **The calibration window restarts every run.** `SelfModelRuntime::new` builds an empty
   `CalibrationWindow::new(model.version)` (`crates/roko-cli/src/graph_task_dispatch/self_model.rs:248`); the saved
   state keeps only the model's outcomes (`crates/roko-learn/src/self_model/model.rs`). The gate needs n = 100 labelled
   outcomes of one predictor version (`crates/roko-learn/src/self_model/gate.rs:27`, `:177`), and `routes()` needs it
   eligible, so even `mode = "active"` cannot route unless 100 attempts settle within a single run, which ordinary plan
   runs never reach.
2. **No calibration export.** `write_calibration` (`gate.rs:281`) is called only by its test (`:441`). The plan runner
   saves `state-v1.json` at run end but never writes `.roko/learn/self-model/calibration.json`, so the `ev.m3.*` values
   never reach S06, and the dashboard's `self_model.calibration` mirror (`crates/roko-core/src/dashboard_snapshot.rs:672`,
   6134) watches a file nothing writes (paper D7 A.8: "calibration.json never written outside tests").

## Why it matters

Goal `cybernetic` (S04 §4.11, §7.4). NEW-FINDINGS finding 14. With shadow on by default (gap-9aca5d), calibration is the
measurement that decides whether M3 may ever be promoted (decision 6102 (3): bounds over the last 100). Stage 3 of the
epic.

## Where

- `crates/roko-cli/src/graph_task_dispatch/self_model.rs::SelfModelRuntime` (`new`, `save`, `gate`).
- `crates/roko-learn/src/self_model/gate.rs::CalibrationWindow`, `write_calibration`, `evaluate`.
- `crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan_body`: the "Persist the self-model (S04, 6129)"
  block at run end.

## Current state

Gate, metrics and breaker are done and tested (6122). Only persistence and the export call are missing.

## Plan

1. Persist the window with the state: serialize it next to `state-v1.json` (or inside a versioned state), keyed by
   predictor version; a run loads it, and a version change starts a fresh window, as the gate requires.
2. At run end (non-frozen runs only), after `runtime.save()`, evaluate the gate and call `write_calibration` with the
   report.
3. Tests `calibration_window_survives_across_runs` (two runs of 60 labelled attempts each give a window of 120, capped
   at the gate's W) and `plan_run_writes_calibration_json` (a fake-provider run in shadow leaves the file with the
   `ev.m3.*` fields).

## Done when

Both `[[verify]]` commands pass.

## Notes

- gap-e464e6 (L1 forecasts swing between 0.03 and 0.97 once warm) will show up as poor calibration; that is expected
  and is that item's fix.
- The live shadow acceptance of 6128 (a prediction for 95% of routed attempts) and the 200-attempt active run of 6130
  were dropped without follow-up (finding 29, another agent's); this item only makes the data exist.
