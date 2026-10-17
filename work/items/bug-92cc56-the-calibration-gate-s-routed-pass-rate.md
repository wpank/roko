+++
id = "bug-92cc56"
kind = "bug"
title = "The calibration gate's routed-pass-rate bound has no routed outcome until the self-model routes, and it routes only once the gate is eligible, so M3 can never become eligible"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/self_model"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "gap-4c531f worker, 2026-10-17: self_model_gate_window_follows_n_min reaches eligibility only with outcomes marked routed by hand"
discovered_from = "gap-4c531f"
anchors = ["crates/roko-learn/src/self_model/gate.rs::CalibrationGate::evaluate", "crates/roko-learn/src/self_model/gate.rs::CalibrationWindow", "crates/roko-cli/src/runtime_feedback/self_model.rs::SelfModelOutcomeSink", "crates/roko-cli/src/graph_task_dispatch/self_model.rs::active_start"]
lane = "rust-hot"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = ["gap-4c531f", "gap-b38ed2", "gap-76d411"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn calibration_gate_can_become_eligible_in_shadow' crates/ && cargo test -p roko-learn calibration_gate_can_become_eligible_in_shadow"
+++

## Problem

The calibration gate can never become eligible, in shadow or active mode, however many well-calibrated outcomes its
window holds. One of its bounds is the pass rate of the attempts the self-model routed, ≥ π* − 0.05
(`ev.m3.route_pass`, S04 §4.7). `CalibrationGate::evaluate` takes it from the window's outcomes whose `routed` flag is
set, and with none it is `None`, which fails the bound: "routed pass rate below π* − 0.05: unknown".

- An outcome is routed only when the self-model chose the attempt's start rung: `SelfModelOutcomeSink::on_event` sets
  `routed` from `ladder.reason == LadderReason::SelfModel`.
- In shadow mode the self-model chooses nothing, so no outcome is routed.
- In active mode it chooses a start rung only while the gate is eligible: `active_start` needs `gate.eligible`, and so
  does `SelfModelRuntime::routes`.
- The offline fit sets `routed: false` on every outcome (`fit_offline`), so it can never report `eligible` either.

So no window ever holds a routed outcome, the bound always fails, and `[self_model] mode = "active"` can never act.

How to see it: settle any number of sharp, correct forecasts through the outcome sink in shadow (or active) mode and
read `gate()` or `.roko/learn/self-model/calibration.json`: `reasons` always lists the routed pass rate as unknown.
gap-4c531f's test `self_model_gate_window_follows_n_min` reaches eligibility only by marking its outcomes routed by
hand.

## Why it matters

Goal `cybernetic`, spec-7a71d6 stage 3. The gate is what lets active mode act. gap-b38ed2 keeps the window across
runs and gap-4c531f lets `n_min` shorten it; this bound still keeps every window ineligible, so the calibration
export reports `eligible: false` for a reason no data can fix, and policy (b)'s start rung, early climbs and
post-pass steps never run outside tests.

## Where

- `crates/roko-learn/src/self_model/gate.rs::CalibrationGate::evaluate` and `CalibrationWindow::route_pass`: the
  bound and the outcomes it reads.
- `crates/roko-cli/src/runtime_feedback/self_model.rs::SelfModelOutcomeSink`: where `routed` is set.
- `crates/roko-cli/src/graph_task_dispatch/self_model.rs::active_start`: routes only when the gate is eligible.
- `crates/roko-cli/src/commands/learn_self_model.rs::fit_offline`: every outcome unrouted.

## Current state

Open. Found while doing gap-4c531f (2026-10-17). The bound and the eligibility rule are as 6122 and 6130 built them;
nothing has changed them since.

## Plan

A design choice; options:

1. Hold the bound only once the self-model has routed something: an unknown routed pass rate fails nothing while no
   outcome is routed, and S06 watches `ev.m3.route_pass` once it routes. Smallest; the first routed attempts then
   run on calibration alone.
2. In shadow, count an attempt as routed for this bound when it ran on the rung the self-model would have chosen
   (`would_choose == default` in its prediction row): the pass rate of the attempts it would have routed the same
   way. Measures the self-model's own choices on attempts that ran them, at the cost of a selection bias towards
   attempts where it agreed with the ladder.
3. Keep the bound and start active mode some other way (a person's promotion that waives it for the first W routed
   outcomes, say), and document that.

Whichever is chosen, `fit_offline` should score the same way.

## Done when

`calibration_gate_can_become_eligible_in_shadow` (the `[[verify]]` command) passes: a window of W well-calibrated
outcomes settled in shadow mode, none of them routed, makes the gate eligible under the chosen rule.

## Notes

The circuit breaker (S04 §4.11.5) does not read the routed pass rate. Per decision 6102 the gate only makes the model
eligible; a person promotes it.
