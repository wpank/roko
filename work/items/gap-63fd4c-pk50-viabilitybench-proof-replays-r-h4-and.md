+++
id = "gap-63fd4c"
kind = "gap"
title = "PK50 ViabilityBench proof: Replays R-H4 and R-M3 on S04's prequential replay"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
rank = 50
size = "S"
subsystem = ["benchmarks/viabilitybench/analysis"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK50"
anchors = ["benchmarks/viabilitybench/analysis"]
lane = "bench"
parent = "spec-abbc62"
links = { depends_on = ["gap-2ca903", "gap-62e1b9", "gap-d90ef6", "gap-7ec3ef"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_r_h4_replay_is_deterministic' benchmarks/viabilitybench/analysis/test_replay_h4.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_replay_h4.py -k test_r_h4_replay_is_deterministic -q"
+++

## Problem

This package delivers 1 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK50, slice 33xx, phase 6), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3355 | S | p2 | Replays R-H4 and R-M3 on S04's prequential replay | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3355-replays-r-h4-and-r-m3.md` |

## Why it matters

Phase 6: M3 calibrated self-model (S04). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3300-the-proof-viabilitybench-pilots-log1-and-head-to-head.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/analysis/replay_h4.py`, `benchmarks/viabilitybench/analysis/test_replay_h4.py`.

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

- Waits on: PK30 (gap-2ca903), PK47 (gap-62e1b9), PK48 (gap-d90ef6), PK49 (gap-7ec3ef).
- Suggested model: sonnet.

## Progress

- 3355: implemented at `fd634a1c0`. New `analysis/replay_h4.py`, a `replay_runner` adapter: R-H4 hands LOG1's
  own run records and `roko learn self-model replay`'s trace lines to `analysis.econ.build` (PK49), reusing its
  VS/CPR/pass^k/bootstrap math, then checks SC2 (a policy's `cpr_usd` <= 0.85x the best static cheap arm's and
  <= H4-B1's). R-M3 scores Brier/BSS, ECE (10 equal-mass bins) and AUROC from `{attempt_key, forecast, outcome}`
  rows, in `fig_f6_reliability.py`'s own bin shape. Both halves need inputs from mechanisms outside this slice
  (the self-model replay's traces, S04's own forecast/outcome pairs); without them each reports
  `evaluated: false` with why, mirroring `replay_h5.py`'s own graceful degradation. Built and tested entirely on
  a LOG1-shaped fixture (no pilot has run, so no real data was available or used); the item's verify passes, and
  `replay_runner.run` gives byte-identical canonical output across two runs at one seed. Regression-checked
  against `test_replay_runner.py`, `test_replay_h5.py` and `test_econ.py` (all green).
  A natural follow-on once Pilots A/B or LOG1 produce real traces/forecasts: gap-8a26fc ("run the M3 replay on
  the pilot matrix") would exercise this adapter's `--traces`/`--forecasts` path end to end; not attempted here
  per this wave's "no live calls, no spend."
