+++
id = "gap-4a5109"
kind = "gap"
title = "PK57 Specs a cheap model can execute: H3 pilot: 10 instances under the locked pre-registration (+2 more)"
status = "open"
triage = "verified"
severity = "p3"
goal = "proof"
rank = 57
size = "L"
hold = "waits on Will's deferred decision(s) 3346, 7101 (tmp/backlog/2026-10-02-complete-and-wire/DECISIONS.md)"
subsystem = ["benchmarks/viabilitybench/experiments"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK57"
anchors = ["benchmarks/viabilitybench", "benchmarks/viabilitybench/experiments"]
lane = "bench"
parent = "spec-c3abc8"
links = { depends_on = ["gap-5ebb4f", "gap-1149aa", "gap-eb1aa3", "gap-e120a1", "gap-daeaa9", "gap-5ddf9b", "gap-2ca903", "gap-7c9a9c", "gap-e9218e", "gap-a21195", "gap-fd96c8"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'prereg_id' benchmarks/viabilitybench/reports/h3-pilot/summary.md && grep -q 'manipulation check: passed' benchmarks/viabilitybench/reports/h3-pilot/summary.md"

[[verify]]
command = "grep -q 'interaction' benchmarks/viabilitybench/reports/h3/summary.md && grep -q 'recovered gap' benchmarks/viabilitybench/reports/h3/summary.md"

[[verify]]
command = "grep -qw 'def test_uplift_table_has_provenance' benchmarks/viabilitybench/specops/tests/test_uplift.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/specops/tests/test_uplift.py -q"
+++

## Problem

This package delivers 3 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK57, slice 32xx, phase 7), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3238 | M | p3 | H3 pilot: 10 instances under the locked pre-registration | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3238-h3-pilot-ten-instances.md` |
| 2 | 3239 | M | p3 | H3 full run in LOG1 block C, with its report | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3239-h3-full-run-in-log1-block-c.md` |
| 3 | 3241 | S | p3 | Fit the refine-uplift table on H3's refined arm | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3241-fit-refine-uplift-table-on-h3.md` |

## Why it matters

Phase 7: M4 random deep audits (S05). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3200-specs-a-cheap-model-can-execute.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/experiments/h3_pilot.toml`, `benchmarks/viabilitybench/reports/h3-pilot/summary.md`, `benchmarks/viabilitybench/reports/h3/summary.md`, `benchmarks/viabilitybench/specops/tests/test_uplift.py`, `benchmarks/viabilitybench/specops/uplift.py`.

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

- Waits on: PK20 (gap-5ebb4f), PK21 (gap-1149aa), PK23 (gap-eb1aa3), PK24 (gap-e120a1), PK25 (gap-daeaa9), PK27 (gap-5ddf9b), PK30 (gap-2ca903), PK31 (gap-7c9a9c), PK37 (gap-e9218e), PK51 (gap-a21195), PK56 (gap-fd96c8).
- On hold until Will takes the deferred decision(s) 3346, 7101 (spend or a public release); see `DECISIONS.md`.
- Suggested model: sonnet.
