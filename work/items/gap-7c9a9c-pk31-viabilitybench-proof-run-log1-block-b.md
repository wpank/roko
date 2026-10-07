+++
id = "gap-7c9a9c"
kind = "gap"
title = "PK31 ViabilityBench proof: Run LOG1 block B: cheap_direct, 660 billed runs (+2 more)"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
rank = 31
size = "L"
hold = "waits on Will's deferred decision(s) 3346 (tmp/backlog/2026-10-02-complete-and-wire/DECISIONS.md)"
subsystem = ["benchmarks/viabilitybench/experiments"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK31"
anchors = ["benchmarks/viabilitybench"]
lane = "bench"
parent = "spec-fef7c5"
links = { depends_on = ["gap-5ebb4f", "gap-daeaa9", "gap-2ca903"], blocks = [], related = ["gap-154f93", "gap-c33709", "q-ab27d3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f benchmarks/viabilitybench/reports/log1/block_b/metrics.json && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/report.py --check benchmarks/viabilitybench/reports/log1/block_b"

[[verify]]
command = "test -f benchmarks/viabilitybench/reports/log1/subscription/metrics.json && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/report.py --check benchmarks/viabilitybench/reports/log1/subscription"

[[verify]]
command = "test -f benchmarks/viabilitybench/external/swebench/slice-v1.jsonl && python3 -c \"import sys; n=sum(1 for l in open('benchmarks/viabilitybench/external/swebench/slice-v1.jsonl') if l.strip()); sys.exit(0 if n == 60 else 1)\""
+++

## Problem

This package delivers 3 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK31, slice 33xx, phase 3), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3349 | M | p1 | Run LOG1 block B: cheap_direct, 660 billed runs | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3349-run-log1-block-b-cheap-direct.md` |
| 2 | 3352 | M | p1 | Run LOG1's subscription cells: fd_claude core and pass^5, then fd_claude_lite and fd_codex | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3352-run-log1-subscription-cells.md` |
| 3 | 3353 | S | p2 | Run the P1-ext contamination probe (LOG1 block F) and freeze slice-v1.jsonl | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3353-run-p1-ext-probe-and-freeze-slice.md` |

## Why it matters

Phase 3: golden-path proof. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3300-the-proof-viabilitybench-pilots-log1-and-head-to-head.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/external/swebench/selection-log.jsonl`, `benchmarks/viabilitybench/external/swebench/slice-v1.jsonl`, `benchmarks/viabilitybench/reports/log1/block_b/`, `benchmarks/viabilitybench/reports/log1/subscription/`.

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

- Waits on: PK20 (gap-5ebb4f), PK25 (gap-daeaa9), PK30 (gap-2ca903).
- On hold until Will takes the deferred decision(s) 3346 (spend or a public release); see `DECISIONS.md`.
- Existing work items this package covers or touches: gap-154f93, gap-c33709, q-ab27d3. When its tasks are done, close those whose verify then passes.
- Suggested model: sonnet.
