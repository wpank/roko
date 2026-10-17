+++
id = "gap-4c531f"
kind = "gap"
title = "[self_model] n_min sets nothing: the calibration gate's window is the constant WINDOW = 100, so a small workspace can't opt into a shorter one"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/self_model"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "tmp/hermes-roko-v6/findings/learning-research.md (learning-research, 2026-10-16)"
anchors = ["crates/roko-learn/src/self_model/gate.rs::WINDOW", "crates/roko-learn/src/self_model/gate.rs::CalibrationGate::evaluate", "crates/roko-core/src/config/self_model.rs::SelfModelConfig", "crates/roko-cli/src/graph_task_dispatch/self_model.rs::SelfModelRuntime"]
links = { depends_on = [], blocks = [], related = ["gap-51d2ca"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn self_model_gate_window_follows_n_min' crates/roko-cli/ && cargo test -p roko-cli self_model_gate_window_follows_n_min"
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
