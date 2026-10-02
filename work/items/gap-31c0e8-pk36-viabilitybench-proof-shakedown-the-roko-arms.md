+++
id = "gap-31c0e8"
kind = "gap"
title = "PK36 ViabilityBench proof: Shakedown: the Roko arms against the live-run defects, offline, before any paid Roko run (+2 more)"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
rank = 36
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK36"
anchors = ["benchmarks/viabilitybench/analysis/test_analysis.py", "benchmarks/viabilitybench/driver/stub_provider.py"]
lane = "bench"
parent = "spec-446a41"
links = { depends_on = ["gap-625195", "gap-e00238", "gap-f548c1", "gap-cc5051", "gap-de0b87", "gap-5ebb4f", "gap-1149aa", "gap-943046"], blocks = [], related = ["gap-c33709", "gap-d9e9fe"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -x target/debug/roko && test \"$(grep -c 'def test_shakedown_' benchmarks/viabilitybench/driver/test_shakedown.py)\" -ge 8 && VB_REQUIRE_REAL_ROKO=1 benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_shakedown.py -k shakedown -q"

[[verify]]
command = "test -f benchmarks/viabilitybench/reports/pilot_c/metrics.json && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/report.py --check benchmarks/viabilitybench/reports/pilot_c"

[[verify]]
command = "grep -qw 'def test_pilot_page_shows_the_ladder_arm_and_the_v7_bar' benchmarks/viabilitybench/analysis/test_analysis.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_analysis.py -k test_pilot_page_shows_the_ladder_arm_and_the_v7_bar -q"
+++

## Problem

This package delivers 3 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK36, slice 33xx, phase 4), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3314 | M | p1 | Shakedown: the Roko arms against the live-run defects, offline, before any paid Roko run | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3314-shakedown-roko-arms-against-live-run-defects.md` |
| 2 | 3315 | S | p1 | Pilot C: roko_ladder on the pilot's 20 tasks x 3 seeds | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3315-pilot-c-roko-ladder-on-the-pilot-tasks.md` |
| 3 | 3316 | S | p1 | The pilot page shows the ladder arm and the V7 bar per level | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3316-pilot-page-shows-ladder-arm-and-v7-bar.md` |

## Why it matters

Phase 4: loops re-closed (S02). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3300-the-proof-viabilitybench-pilots-log1-and-head-to-head.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/analysis/pilot_page.py`, `benchmarks/viabilitybench/analysis/test_analysis.py`, `benchmarks/viabilitybench/driver/stub_provider.py`, `benchmarks/viabilitybench/driver/test_shakedown.py`, `benchmarks/viabilitybench/reports/pilot_c/`.

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

- Waits on: PK01 (gap-625195), PK02 (gap-e00238), PK07 (gap-f548c1), PK09 (gap-cc5051), PK19 (gap-de0b87), PK20 (gap-5ebb4f), PK21 (gap-1149aa), PK35 (gap-943046).
- Existing work items this package covers or touches: gap-c33709, gap-d9e9fe. When its tasks are done, close those whose verify then passes.
- Suggested model: sonnet.
