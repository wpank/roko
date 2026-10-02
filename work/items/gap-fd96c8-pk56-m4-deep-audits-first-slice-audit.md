+++
id = "gap-fd96c8"
kind = "gap"
title = "PK56 M4 deep audits: First slice: audit at least 60 recorded cheap-arm passes and publish the replay report…"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 56
size = "S"
hold = "waits on Will's deferred decision(s) 7101 (tmp/backlog/2026-10-02-complete-and-wire/DECISIONS.md)"
subsystem = ["benchmarks/viabilitybench/audit"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK56"
anchors = ["benchmarks/viabilitybench"]
lane = "bench"
parent = "spec-c3abc8"
links = { depends_on = ["gap-46fd19", "gap-eb1aa3", "gap-c06ff3", "gap-dff960", "gap-3d37e8", "gap-33eec2"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f benchmarks/viabilitybench/reports/s05-slice1/REPORT.md && grep -q 'theta_census' benchmarks/viabilitybench/reports/s05-slice1/REPORT.md && grep -q 'n_eff' benchmarks/viabilitybench/reports/s05-slice1/REPORT.md"
+++

## Problem

This package delivers 1 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK56, slice 71xx, phase 7), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 7111 | S | p2 | First slice: audit at least 60 recorded cheap-arm passes and publish the replay report (S05 task 3) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7111-first-slice-audit-60-cheap-arm-passes.md` |

## Why it matters

Phase 7: M4 random deep audits (S05). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7100-m4-random-deep-audits.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/reports/s05-slice1/REPORT.md`, `benchmarks/viabilitybench/reports/s05-slice1/summary.json`.

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

- Waits on: PK22 (gap-46fd19), PK23 (gap-eb1aa3), PK28 (gap-c06ff3), PK53 (gap-dff960), PK54 (gap-3d37e8), PK55 (gap-33eec2).
- On hold until Will takes the deferred decision(s) 7101 (spend or a public release); see `DECISIONS.md`.
- Suggested model: opus.
