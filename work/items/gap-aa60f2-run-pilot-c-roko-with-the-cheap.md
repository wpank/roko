+++
id = "gap-aa60f2"
kind = "gap"
title = "Run Pilot C: Roko with the cheap-model ladder on the pilot tasks, task 3315"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
hold = "needs Pilot A's G0 checks first (its key file) and a live paid run from Will's machine, within the approved $21 for pilots A-C"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "tmp/backlog/2026-10-02-complete-and-wire 3315 (blocked in wave 7, PK36)"
discovered_from = "gap-31c0e8"
anchors = ["benchmarks/viabilitybench/experiments"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["gap-31c0e8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f benchmarks/viabilitybench/reports/pilot_c/metrics.json && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/report.py --check benchmarks/viabilitybench/reports/pilot_c"
+++

## Problem

Task 3315 of PK36 (gap-31c0e8, merged in wave 7) runs Pilot C: Roko with the cheap models plus escalation on the
pilot's 20 hidden-test tasks x 3 seeds, beside Pilot A's `cheap_direct` and Pilot B's `fd_claude`. It needs a live,
paid `vb campaign --manifest experiments/pilot_c.toml --allow-network` run from Will's machine, and wave 7 made no
live calls, so `reports/pilot_c/` doesn't exist.

## Why it matters

It gives the first three-arm V7 evidence (descriptive, not H1): how often the ladder escalates, which rung serves
each attempt, and the cost split by class.

## Where

`benchmarks/viabilitybench/experiments/pilot_c.toml` (3313); raw results in `$VB_RESULTS/PILOT-C/`; the committed
summary bundle under `benchmarks/viabilitybench/reports/pilot_c/` (D4).

## Current state

Will approved pilots A-C within $21 in total (2026-10-02). Pilot A waits on its key file, and 3315's plan runs Pilot
C only after Pilot A's G0 direct-arm checks pass. PK36's offline shakedown (3314,
`driver/test_shakedown.py`) is merged.

## Plan

1. After Pilot A's G0 checks pass, run Pilot C as task 3315 says, with the cap from the estimate, one runner.
2. `vb report --experiment PILOT-C --bundle reports/pilot_c`, then `report.py --check`; commit the bundle.

## Done when

- [ ] The `[[verify]]` command passes.

## Notes

- Full spec: `tmp/backlog/2026-10-02-complete-and-wire/3315-pilot-c-roko-ladder-on-the-pilot-tasks.md`.
- Left PK36's package item at gate 7a (2026-10-03), as 3236 and 6107 left theirs.
