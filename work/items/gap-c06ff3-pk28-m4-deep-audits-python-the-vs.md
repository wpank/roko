+++
id = "gap-c06ff3"
kind = "gap"
title = "PK28 M4 deep audits: Python: the vs.label schema, the audit estimators and the lottery replay (S05 task 2)"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 28
size = "M"
subsystem = ["benchmarks/viabilitybench/audit"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK28"
anchors = ["benchmarks/viabilitybench", "benchmarks/viabilitybench/schema"]
lane = "bench"
parent = "spec-fef7c5"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_hajek_ci_covers_theta_in_1000_replays' benchmarks/viabilitybench/audit/tests/test_estimate.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/audit/tests/test_estimate.py -q"
+++

## Problem

This package delivers 1 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK28, slice 71xx, phase 3), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 7107 | M | p2 | Python: the vs.label schema, the audit estimators and the lottery replay (S05 task 2) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7107-python-vs-label-estimators-and-lottery-replay.md` |

## Why it matters

Phase 3: golden-path proof. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7100-m4-random-deep-audits.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/audit/__init__.py`, `benchmarks/viabilitybench/audit/estimate.py`, `benchmarks/viabilitybench/audit/fixtures/estimators.json`, `benchmarks/viabilitybench/audit/lottery.py`, `benchmarks/viabilitybench/audit/replay.py`, `benchmarks/viabilitybench/audit/tests/test_estimate.py`, `benchmarks/viabilitybench/schema/vs-label.schema.json`.

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

- Waits on: nothing.
- Suggested model: opus.

## Progress

- 7107: implemented at 38061d398
