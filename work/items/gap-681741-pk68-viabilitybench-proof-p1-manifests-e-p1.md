+++
id = "gap-681741"
kind = "gap"
title = "PK68 ViabilityBench proof: P1 manifests: E-P1-live, E-P1-ext, E-fd-api, E-drift and E-T5-loo"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
rank = 68
size = "S"
hold = "waits on Will's deferred decision(s) 3333, 3346 (tmp/backlog/2026-10-02-complete-and-wire/DECISIONS.md)"
subsystem = ["benchmarks/viabilitybench/experiments"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK68"
anchors = ["benchmarks/viabilitybench/experiments"]
lane = "bench"
parent = "spec-635697"
links = { depends_on = ["gap-057cc7", "gap-7c9a9c", "gap-a42df2", "gap-ed1a08"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_p1_manifests_validate_and_name_their_lines' benchmarks/viabilitybench/experiments/test_p1.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/experiments/test_p1.py -k test_p1_manifests_validate_and_name_their_lines -q"
+++

## Problem

This package delivers 1 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK68, slice 33xx, phase 8), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3362 | S | p1 | P1 manifests: E-P1-live, E-P1-ext, E-fd-api, E-drift and E-T5-loo | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3362-p1-manifests-live-ext-fd-api-drift-t5.md` |

## Why it matters

Phase 8: M1 controller and guarded commit (S06). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3300-the-proof-viabilitybench-pilots-log1-and-head-to-head.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/experiments/p1_drift.toml`, `benchmarks/viabilitybench/experiments/p1_ext.toml`, `benchmarks/viabilitybench/experiments/p1_fd_api.toml`, `benchmarks/viabilitybench/experiments/p1_live.toml`, `benchmarks/viabilitybench/experiments/p1_t5_loo.toml`, `benchmarks/viabilitybench/experiments/test_p1.py`.

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

- Waits on: PK26 (gap-057cc7), PK31 (gap-7c9a9c), PK45 (gap-a42df2), PK67 (gap-ed1a08).
- On hold until Will takes the deferred decision(s) 3333, 3346 (spend or a public release); see `DECISIONS.md`.
- Suggested model: sonnet.
