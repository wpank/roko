+++
id = "gap-4c531f"
kind = "gap"
title = "[self_model] n_min sets nothing: the calibration gate's window is the constant WINDOW = 100, so a small workspace can't opt into a shorter one"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/self_model"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "24986d8b3"
source = "tmp/hermes-roko-v6/findings/learning-research.md (learning-research, 2026-10-16)"
anchors = ["crates/roko-learn/src/self_model/gate.rs::WINDOW", "crates/roko-learn/src/self_model/gate.rs::CalibrationGate::evaluate", "crates/roko-core/src/config/self_model.rs::SelfModelConfig", "crates/roko-cli/src/graph_task_dispatch/self_model.rs::SelfModelRuntime"]
links = { depends_on = [], blocks = [], related = ["gap-51d2ca"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn self_model_gate_window_follows_n_min' crates/roko-cli/ && cargo test -p roko-cli self_model_gate_window_follows_n_min"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T03:30:09Z"
commit = "24986d8b3"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-17T02:10:41Z"
forced = false
evidence = "24986d8b3 makes [self_model] n_min the calibration gate's window W. In roko-learn gate.rs, CalibrationGate gains `window` (CalibrationGate::of builds the gate from [self_model] target and n_min, 100 by default); evaluate() scores the last W outcomes, the routed pass rate included, and reports \"n = 9, below the window of 10\"; GateReport and calibration.json carry `window`. A window keeps max(W, 100) outcomes (CalibrationWindow::keeping and capacity, a run setting that window-v1.json does not store; load and save_outcomes take it), so the breaker keeps its last 50 and a W above 100 fits. SelfModelRuntime builds its gate from its settings and holds its window, its unsaved outcomes and the window file at that capacity, and `roko learn self-model fit` scores with the workspace's gate and reports `window`. The bounds are unchanged; gate.rs's module doc and n_min's doc say a small W makes them noisy. The literal verify, `grep -rqw 'fn self_model_gate_window_follows_n_min' crates/roko-cli/ && cargo test -p roko-cli self_model_gate_window_follows_n_min`, passes at 24986d8b3 (76 test binaries, the named test ok): with n_min = 10, nine sharp, correct outcomes fail only for \"n = 9, below the window of 10\", the tenth makes the gate eligible and an active self-model route, the next run loads eligible, and n_min = 100 finds the ten short. Also passing at 24986d8b3: roko-learn self_model::gate (6 tests, among them the new the_gate_scores_a_window_of_its_own_w: W = 10 eligible on ten outcomes while the window keeps 100, W = 150 keeps 150 in memory and in the file), roko-cli --lib self_model (13), plan_run_writes_calibration_json, calibration_window_survives_across_runs, and the learn_self_model tests, where n_min = 9 lets the fit's gate score the fixture's nine units. `cargo clippy -p roko-learn -p roko-core -p roko-cli --no-deps -- -D warnings` is clean, and the branch is formatted with `cargo +nightly fmt --all`. Filed bug-92cc56: the routed-pass-rate bound needs routed outcomes, which only an eligible active self-model makes, so the new test marks its outcomes routed by hand."
+++

## Problem

`[self_model] n_min` sets nothing. The config documents it as "outcomes the calibration window needs before active
mode may act" (default 100), but only config validation and tests read it. The gate's window is the constant
`gate.rs::WINDOW = 100`, used by `CalibrationGate::evaluate` (`n < WINDOW` makes the gate ineligible) and
`CalibrationWindow::scored`. A small workspace cannot opt into a shorter window.

## Why it matters

The round-v6 pitch says roko gets cheaper with use, and part of that is learning the start rung. The self-model already
learns it in shadow: over three similar one-task plans, P(verified | gpt-oss-120b) fell from 0.475 to 0.123 after run 1
failed twice on it, and from run 2 on it "would start on" glm-4.7 (`runs/*/predictions.jsonl`), while the ladder kept
starting on the cheap rung. `roko learn self-model fit --offline` says "calibration: insufficient_n (n = 7 < 100)", and
`n_min` can't lower that bar for a demo or a small workspace (learning-research, round v6, at 47b3cdd20).

## Where

- `crates/roko-learn/src/self_model/gate.rs::WINDOW`, `CalibrationGate::evaluate`, `CalibrationWindow::scored`.
- `crates/roko-core/src/config/self_model.rs::SelfModelConfig` (`n_min`).
- `crates/roko-cli/src/graph_task_dispatch/self_model.rs::SelfModelRuntime` (builds the gate).

## Plan

Make `[self_model] n_min` the gate's window W: give `CalibrationGate` a window field, set it from
`SelfModelConfig::n_min` in `SelfModelRuntime::gate`, and keep 100 as the default. The ECE, calibration-in-the-large,
Brier skill and AUROC bounds stay as they are; document that a small W makes those bounds noisy.

## Done when

`self_model_gate_window_follows_n_min` passes (the `[[verify]]` command): with `n_min = 10`, ten well-calibrated outcomes
make the gate eligible, and nine do not.

## Notes

Split out of gap-b38ed2 on 2026-10-16 so that item could close against its original two verifies (its window-restart
and calibration.json fixes landed on `work/gap-b38ed2`).
