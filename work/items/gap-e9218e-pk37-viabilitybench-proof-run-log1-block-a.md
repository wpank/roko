+++
id = "gap-e9218e"
kind = "gap"
title = "PK37 ViabilityBench proof: Run LOG1 block A: roko_fixed on four cheap models, 960 billed runs (+1 more)"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
rank = 37
size = "M"
hold = "waits on Will's deferred decision(s) 3346 (tmp/backlog/2026-10-02-complete-and-wire/DECISIONS.md)"
subsystem = ["benchmarks/viabilitybench/experiments"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK37"
anchors = ["benchmarks/viabilitybench"]
lane = "bench"
parent = "spec-446a41"
links = { depends_on = ["gap-625195", "gap-e00238", "gap-f548c1", "gap-cc5051", "gap-de0b87", "gap-46fd19", "gap-2ca903", "gap-943046", "gap-31c0e8"], blocks = [], related = ["gap-644040"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f benchmarks/viabilitybench/reports/log1/block_a/metrics.json && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/report.py --check benchmarks/viabilitybench/reports/log1/block_a"

[[verify]]
command = "test -f benchmarks/viabilitybench/reports/log1/block_de/metrics.json && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/report.py --check benchmarks/viabilitybench/reports/log1/block_de"
+++

## Problem

This package delivers 2 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK37, slice 33xx, phase 4), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3350 | M | p1 | Run LOG1 block A: roko_fixed on four cheap models, 960 billed runs | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3350-run-log1-block-a-roko-fixed.md` |
| 2 | 3351 | S | p1 | Run LOG1 blocks D and E: convention_flip rows and F8 honeypots | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3351-run-log1-blocks-d-and-e.md` |

## Why it matters

Phase 4: loops re-closed (S02). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3300-the-proof-viabilitybench-pilots-log1-and-head-to-head.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/reports/log1/block_a/`, `benchmarks/viabilitybench/reports/log1/block_de/`.

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

- Waits on: PK01 (gap-625195), PK02 (gap-e00238), PK07 (gap-f548c1), PK09 (gap-cc5051), PK19 (gap-de0b87), PK22 (gap-46fd19), PK30 (gap-2ca903), PK35 (gap-943046), PK36 (gap-31c0e8).
- On hold until Will takes the deferred decision(s) 3346 (spend or a public release); see `DECISIONS.md`.
- Existing work items this package covers or touches: gap-644040. When its tasks are done, close those whose verify then passes.
- Suggested model: sonnet.
