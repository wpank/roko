+++
id = "bug-4b3fdd"
kind = "bug"
title = "A calibration window of one class (every outcome a pass) leaves Brier skill and AUROC undefined, so the gate drops an active self-model that is right every time back to the ladder"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/self_model"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "d8943f7e2"
source = "round-v6 learning-research start-rung re-run (2026-10-17)"
discovered_from = "bug-92cc56"
anchors = ["crates/roko-learn/src/self_model/gate.rs::CalibrationGate::evaluate"]
lane = "rust-hot"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = ["gap-3f36a6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_one_class_window_is_judged_by_its_calibration' crates/roko-learn/ && cargo test -p roko-learn a_one_class_window_is_judged_by_its_calibration"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T20:35:58Z"
commit = "4e930f712"
forced = false
evidence = "4e930f712: with one class in the window, Brier skill and AUROC fail nothing while ECE and calibration-in-the-large hold. a_one_class_window_is_judged_by_its_calibration failed first (12 passes forecast at 0.97 ineligible on Brier skill and AUROC unknown) and passes; 12 passes forecast at 0.6 stay ineligible. $0 start-rung re-run (tmp/hermes-roko-v6/learning-research/evidence/start-rung/AB): no fallback to the ladder from a one-class window (before: tasks 18 and 30 fell back to cheap). The [[verify]] passes on the merged tree d8943f7e2."
+++

## Problem

When every outcome in the calibration gate's window has the same result, all passes or all failures, Brier skill
and AUROC are undefined. `CalibrationGate::evaluate` then fails both bounds as "unknown", and the gate drops
eligibility, although the forecasts are calibrated and there is nothing to discriminate.

An active self-model that starts every task on the rung that passes produces exactly that window. In
learning-research's $0 start-rung re-run (round v6, binary roko-92cc56-3f36a6-merged, `n_min = 12`), the window
held 12 straight routed passes after tasks 17 and 29. The gate went ineligible, and tasks 18 and 30 fell back to
the ladder, started on cheap and climbed (3 tries each). The default route holdout put no failing chain in the
window. Evidence: `tmp/hermes-roko-v6/learning-research/evidence/start-rung/merged/table.md`.

## Why it matters

Goal `cybernetic`, the round-v6 pitch's "learns where to start". A self-model that is right every time loses its
eligibility for being right, every W tasks, and each fallback costs a full climb. The bug-92cc56 note named it.

## Where

- `crates/roko-learn/src/self_model/gate.rs::CalibrationGate::evaluate`: the Brier skill and AUROC bounds.

## Current state

Open. Found by learning-research's re-run of the bug-92cc56 fix, 2026-10-17.

## Plan

As with the routed pass rate under bug-92cc56 (option 1): when the window holds one class, Brier skill and AUROC
fail nothing while ECE and calibration-in-the-large hold. A one-class window that is not calibrated stays
ineligible.

## Done when

`a_one_class_window_is_judged_by_its_calibration` (the `[[verify]]` command) passes: twelve passes forecast at 0.97
make the gate eligible, and twelve passes forecast at 0.6 do not. Re-running the start-rung demo's `active` series
shows no fallback to the ladder from a one-class window.
