+++
id = "gap-a42df2"
kind = "gap"
title = "PK45 ViabilityBench proof: requires_live: refuse a live manifest whose loops are not LIVE at its harness_sha (+1 more)"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
rank = 45
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK45"
anchors = ["benchmarks/viabilitybench/driver", "benchmarks/viabilitybench/experiments"]
lane = "bench"
parent = "spec-c6e21b"
links = { depends_on = ["gap-f61823", "gap-daeaa9", "gap-5ddf9b", "gap-1f4bec", "gap-85d176"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_live_manifest_refuses_loops_that_are_not_live' benchmarks/viabilitybench/driver/test_campaign.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_campaign.py -k test_live_manifest_refuses_loops_that_are_not_live -q"

[[verify]]
command = "grep -qw 'def test_live_manifests_name_lines_caps_and_required_loops' benchmarks/viabilitybench/experiments/test_live.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/experiments/test_live.py -k test_live_manifests_name_lines_caps_and_required_loops -q"
+++

## Problem

This package delivers 2 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK45, slice 33xx, phase 5), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3359 | S | p1 | requires_live: refuse a live manifest whose loops are not LIVE at its harness_sha | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3359-requires-live-refuses-loops-not-live.md` |
| 2 | 3361 | S | p2 | Live manifests for H5, H6 and H7 | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3361-live-manifests-h5-h6-h7.md` |

## Why it matters

Phase 5: M2 loop-liveness audit (S03). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3300-the-proof-viabilitybench-pilots-log1-and-head-to-head.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/driver/campaign.py`, `benchmarks/viabilitybench/driver/test_campaign.py`, `benchmarks/viabilitybench/experiments/h5_live.toml`, `benchmarks/viabilitybench/experiments/h6_live.toml`, `benchmarks/viabilitybench/experiments/h7_live.toml`, `benchmarks/viabilitybench/experiments/test_live.py`.

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

- Waits on: PK10 (gap-f61823), PK25 (gap-daeaa9), PK27 (gap-5ddf9b), PK40 (gap-1f4bec), PK44 (gap-85d176).
- Suggested model: sonnet.
