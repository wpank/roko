+++
id = "gap-7ec3ef"
kind = "gap"
title = "PK49 M3 self-model: Compute the M3 economics report from policy replays (+7 more)"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 49
size = "L"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-02
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "cf37d7628"
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
command = "grep -rqw 'fn prediction_record_round_trips_and_lands_in_predictions_jsonl' crates/roko-learn/src/telemetry/ && cargo test -p roko-learn prediction_record_round_trips_and_lands_in_predictions_jsonl"

[[verify]]
command = "grep -rqw 'fn self_model_config_defaults_off' crates/roko-core/src/config/ && cargo test -p roko-core self_model_config_defaults_off"

[[verify]]
command = "grep -rqw 'fn self_model_shadow_writes_predictions_and_keeps_routing' crates/roko-cli/src/ && cargo test -p roko-cli self_model_shadow_writes_predictions_and_keeps_routing"

[[verify]]
command = "grep -rqw 'fn settled_verdict_updates_self_model_state' crates/roko-cli/src/ && cargo test -p roko-cli settled_verdict_updates_self_model_state"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T17:06:48Z"
commit = "cf37d7628"
executor = "claude-agent"
via = "work-batch"
size = "L"
claimed_at = "2026-10-03T07:38:36Z"
forced = false
evidence = "Gate 8c (work/backlog-batch-8c, merged into main as cf37d7628): cargo check --workspace --tests, nightly fmt, cargo clippy --workspace -D warnings (two lints fixed, ea964cad1 and eac374ef0), nextest --lib 7,104 tests over roko-cli, -core, -learn and -neuro (one shadow-mode test compared the arm's model key with the provider slug, fixed in 658adbbed), roko-cli bin 436 passed and the golden-path canaries pass (plan_validate: only bug-2a31bc's two known alias tests fail), roko-learn integration tests pass, the bench analysis suite 87 passed; every [[verify]] passes. PK49 6/8: econ.py, roko learn self-model fit/replay and econ prices, the roko.prediction/1 record, [self_model] (invariant 14), the shadow hook, the outcome sink; 6125 is the held gap-8a26fc and 6130 is gap-d2d750."
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

## Progress

- 6123: implemented at e2f293996 (verify run with the bench venv: 3 passed; the whole analysis suite 77 passed, 1 skipped)
- 6124: implemented at 53e1f170c (cargo verification deferred to the batch gate)
- 6125: blocked: no pilot records exist (Pilot A gap-c33709 and Pilot B gap-327242 are open, and `reports/` holds only `simulation/`); the smoke-test report cannot be made without them
- 6126: implemented at 28a19a301 (a new S01 `AttemptPredictionRecord`: neither existing `PredictionRecord` fits, since one carries outcomes and the other is a Cell's guess)
- 6127: implemented at 8ace7692f (config invariant 14; 13 is PK43's `[learning.audit]`)
- 6128: implemented at 59156dc0a
- 6129: implemented at 0f1da8903
- 6130: blocked: it composes the self-model's rung into S03's single route table (holdout, explore, propensity), which PK43 (gap-c1d920) is wiring into `ModelRouter::decide` in `model_routing.rs` now; the task forbids editing that file in parallel with S03.T11, and a second L-M3 draw would duplicate the table. It can go once PK43 is merged.
- 2026-10-03 (coordinator, gate 8c): 6125 (the replay on the pilot matrix) left this item for the held gap-8a26fc (no pilot records yet), and 6130 (the active self-model's start rung) for gap-d2d750, which follows PK43 now that both are merged.
