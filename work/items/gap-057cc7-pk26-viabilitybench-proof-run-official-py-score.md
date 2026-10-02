+++
id = "gap-057cc7"
kind = "gap"
title = "PK26 ViabilityBench proof: run_official.py: score patches with the official SWE-bench harness (gold 60/60, empty…"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
rank = 26
size = "M"
hold = "waits on Will's deferred decision(s) 3333 (tmp/backlog/2026-10-02-complete-and-wire/DECISIONS.md)"
subsystem = ["benchmarks/viabilitybench/external"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK26"
anchors = ["benchmarks/viabilitybench"]
lane = "bench"
parent = "spec-fef7c5"
links = { depends_on = ["gap-daeaa9"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f benchmarks/viabilitybench/external/swebench/harness-check.json && python3 -c \"import json,sys; d=json.load(open('benchmarks/viabilitybench/external/swebench/harness-check.json')); sys.exit(0 if d['gold']['resolved']==60 and d['empty']['resolved']==0 else 1)\""
+++

## Problem

This package delivers 1 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK26, slice 33xx, phase 3), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3334 | M | p2 | run_official.py: score patches with the official SWE-bench harness (gold 60/60, empty 0/60) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3334-run-official-swe-bench-harness.md` |

## Why it matters

Phase 3: golden-path proof. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3300-the-proof-viabilitybench-pilots-log1-and-head-to-head.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/external/swebench/harness-check.json`, `benchmarks/viabilitybench/external/swebench/run_official.py`, `benchmarks/viabilitybench/external/swebench/test_swebench.py`.

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

- Waits on: PK25 (gap-daeaa9).
- On hold until Will takes the deferred decision(s) 3333 (spend or a public release); see `DECISIONS.md`.
- Suggested model: sonnet.
