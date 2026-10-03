+++
id = "dec-3d5714"
kind = "decision"
title = "S09 §4.1's two-stage bootstrap over-covers (0.975): change to task-only n_h-1 resampling, or accept, before the lock"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/analysis"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "gap-5ddf9b task 3339 (wave 5)"
anchors = ["benchmarks/viabilitybench/analysis/bootstrap.py"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

PK27's task 3339 (gap-5ddf9b, merged in 2347ad858) ran S09's synthetic simulator in full (1,000 campaigns, B = 10,000;
report in `benchmarks/viabilitybench/reports/simulation/`). S09 §4.1's two-stage bootstrap (tasks within family ×
level strata, then seeds within tasks) gives a VS-rate interval coverage of 0.975 ± 0.005, above SC1's 0.93–0.97
band. The difference (0.968) and ratio (0.970) sit at the top of the band; CUPED, the confidence sequences and both
FWERs are in band.

## Why it matters

The pre-registration lock (3345) freezes the analysis. An interval method that over-covers is conservative: it
loses power on H1/H3 at LOG1's planned size.

## Where

S09 §4.1 (untracked: `tmp/cybernetic-harness/specs/S09-*.md`), `benchmarks/viabilitybench/analysis/bootstrap.py`.

## Current state

The cause, per the worker: the second stage counts seed noise twice. Resampling tasks only covers 0.90; resampling
n_h − 1 tasks per stratum covers about 0.95.

## Plan

Will chooses before the lock:
- **(a) Change §4.1** to task-only resampling with n_h − 1 draws per stratum (≈ 0.95 coverage), update
  `bootstrap.py` and re-run 3339's simulator.
- **(b) Accept** the conservative intervals and record the over-coverage in §4.1.

## Done when

The choice is recorded in S09 §4.1, and under (a) the simulator reports VS-rate coverage within 0.93–0.97.

## Notes

- Raised at gate 5b, 2026-10-03.
