+++
id = "gap-50346f"
kind = "gap"
title = "PK46 ViabilityBench proof: Run E-H7-live: the S1 stream and the harmful stream (BL5)"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
rank = 46
size = "S"
hold = "waits on Will's deferred decision(s) 3333, 3346, 3363 (tmp/backlog/2026-10-02-complete-and-wire/DECISIONS.md)"
subsystem = ["benchmarks/viabilitybench/experiments"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK46"
anchors = ["benchmarks/viabilitybench"]
lane = "bench"
parent = "spec-c6e21b"
links = { depends_on = ["gap-943046", "gap-894977", "gap-c2b1a3", "gap-c1d920", "gap-85d176", "gap-a42df2"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f benchmarks/viabilitybench/reports/h7_live/metrics.json && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/report.py --check benchmarks/viabilitybench/reports/h7_live"
+++

## Problem

This package delivers 1 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK46, slice 33xx, phase 5), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3366 | S | p2 | Run E-H7-live: the S1 stream and the harmful stream (BL5) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3366-run-e-h7-live.md` |

## Why it matters

Phase 5: M2 loop-liveness audit (S03). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3300-the-proof-viabilitybench-pilots-log1-and-head-to-head.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/reports/h7_live/`.

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

- Waits on: PK35 (gap-943046), PK38 (gap-894977), PK41 (gap-c2b1a3), PK43 (gap-c1d920), PK44 (gap-85d176), PK45 (gap-a42df2).
- On hold until Will takes the deferred decision(s) 3333, 3346, 3363 (spend or a public release); see `DECISIONS.md`.
- Suggested model: sonnet.
