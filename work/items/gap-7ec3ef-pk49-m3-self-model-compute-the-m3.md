+++
id = "gap-7ec3ef"
kind = "gap"
title = "PK49 M3 self-model: Compute the M3 economics report from policy replays (+7 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 49
size = "L"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK49"
anchors = ["crates/roko-cli/src/commands/learn.rs", "crates/roko-cli/src/commands/mod.rs", "crates/roko-cli/src/dispatch/model_routing.rs", "crates/roko-cli/src/runtime_feedback/mod.rs", "crates/roko-core/src/config/mod.rs", "crates/roko-core/src/config/schema.rs", "crates/roko-learn/src/telemetry/records.rs", "crates/roko-learn/src/telemetry/writer.rs"]
lane = "rust-hot"
parent = "spec-abbc62"
links = { depends_on = ["gap-cc5051", "gap-f61823", "gap-08120e", "gap-5ebb4f", "gap-5ddf9b", "gap-b5caf3", "gap-ac2611", "gap-62e1b9", "gap-d90ef6"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_policy_report_reuses_passk_and_cluster_bootstrap' benchmarks/viabilitybench/analysis/test_econ.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_econ.py -q"

[[verify]]
command = "grep -rqw 'fn learn_self_model_fit_reports_insufficient_n' crates/roko-cli/src/ && cargo test -p roko-cli learn_self_model_fit_reports_insufficient_n"

[[verify]]
command = "test -n \"$(find benchmarks/viabilitybench/reports -name econ-report.json 2>/dev/null)\""

[[verify]]
command = "grep -rqw 'fn prediction_record_round_trips_and_lands_in_predictions_jsonl' crates/roko-learn/src/telemetry/ && cargo test -p roko-learn prediction_record_round_trips_and_lands_in_predictions_jsonl"

[[verify]]
command = "grep -rqw 'fn self_model_config_defaults_off' crates/roko-core/src/config/ && cargo test -p roko-core self_model_config_defaults_off"

[[verify]]
command = "grep -rqw 'fn self_model_shadow_writes_predictions_and_keeps_routing' crates/roko-cli/src/ && cargo test -p roko-cli self_model_shadow_writes_predictions_and_keeps_routing"

[[verify]]
command = "grep -rqw 'fn settled_verdict_updates_self_model_state' crates/roko-cli/src/ && cargo test -p roko-cli settled_verdict_updates_self_model_state"

[[verify]]
command = "grep -rqw 'fn active_self_model_picks_start_rung_and_logs_propensity' crates/roko-cli/src/ && cargo test -p roko-cli active_self_model_picks_start_rung_and_logs_propensity"
+++

## Problem

This package delivers 8 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK49, slice 61xx, phase 6), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 6123 | M | p2 | Compute the M3 economics report from policy replays | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6123-compute-the-m3-economics-report-from-policy-replays.md` |
| 2 | 6124 | S | p2 | Add roko learn self-model fit and replay, and roko learn econ prices | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6124-add-roko-learn-self-model-fit-and-replay-and-roko-learn-econ.md` |
| 3 | 6125 | S | p2 | Run the M3 replay on the pilot matrix and file it as a labelled smoke test | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6125-run-the-m3-replay-on-the-pilot-matrix-and-file-it-as-a.md` |
| 4 | 6126 | S | p2 | Add the roko.prediction/1 record and route it to the run's predictions.jsonl | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6126-add-the-roko-prediction-1-record-and-route-it-to-the-run-s.md` |
| 5 | 6127 | S | p2 | Add the [self_model] config section, off by default | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6127-add-the-self-model-config-section-off-by-default.md` |
| 6 | 6128 | M | p2 | Forecast every routed attempt in shadow mode without changing the route | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6128-forecast-every-routed-attempt-in-shadow-mode-without.md` |
| 7 | 6129 | M | p2 | Update the self-model from settled verdicts, keep its state across runs, and accept late VS labels | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6129-update-the-self-model-from-settled-verdicts-keep-its-state.md` |
| 8 | 6130 | M | p2 | In active mode, let the self-model choose the start rung within the ladder and log its propensity | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6130-in-active-mode-let-the-self-model-choose-the-start-rung.md` |

## Why it matters

Phase 6: M3 calibrated self-model (S04). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6100-epic-m3-calibrated-self-model.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/analysis/econ.py`, `benchmarks/viabilitybench/analysis/test_econ.py`, `benchmarks/viabilitybench/reports/**`, `crates/roko-cli/src/commands/learn.rs`, `crates/roko-cli/src/commands/learn_self_model.rs`, `crates/roko-cli/src/commands/mod.rs`, `crates/roko-cli/src/dispatch/model_routing.rs`, `crates/roko-cli/src/graph_execution/plan_runner.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, `crates/roko-cli/src/graph_task_dispatch/self_model.rs`, `crates/roko-cli/src/runtime_feedback/mod.rs`, `crates/roko-cli/src/runtime_feedback/self_model.rs`, `crates/roko-core/src/config/mod.rs`, `crates/roko-core/src/config/schema.rs`, `crates/roko-core/src/config/self_model.rs`, `crates/roko-learn/src/telemetry/records.rs`, `crates/roko-learn/src/telemetry/writer.rs`.

It also edits the hot file(s) `crates/roko-cli/src/graph_execution/plan_runner.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, which are left out of this item's anchors so that two hot packages can run at once; the coordinator resolves any merge conflict.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: PK09 (gap-cc5051), PK10 (gap-f61823), PK12 (gap-08120e), PK20 (gap-5ebb4f), PK27 (gap-5ddf9b), PK32 (gap-b5caf3), PK34 (gap-ac2611), PK47 (gap-62e1b9), PK48 (gap-d90ef6).
- Suggested model: opus.
- 2026-10-02 (filer-grpD, backlog wave reports, PK96 gap-a6dab7): task 6123 is the `econ.py` producer
  `benchmarks/viabilitybench/analysis/figlib.py`'s "Metric names" section names for `envelope_ratio_r` (and
  probably `cc_<k>`, `envelope_ratio_c`) — `fig_f3_envelope.py:27` ("R_m = VS rate(arm) / VS rate(fd_claude) ...")
  and `tab_t3_headline.py`/`tab_t7_envelope.py` already declare these in their `READS` with nothing emitting them
  yet (confirmed at HEAD: no producer defines `envelope_ratio_r`). This task's own file only mentions "the
  policy-level beta, oracle or Pareto set"; check it names the file `econ.py` and emits these exact metric names
  (per `figlib.py:52-54`'s name/clause contract) so the already-written figure/table scripts resolve without a
  follow-up fix. `regret_cum` is probably task 3340's (replay.py), not this one — see the note on gap-5ddf9b.
