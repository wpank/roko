+++
id = "gap-11cec6"
kind = "gap"
title = "PK69 ViabilityBench proof: Run E-H5-live: H5-A0, H5-A1 and H5-A3 (BL4) (+7 more)"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
rank = 69
size = "L"
hold = "waits on Will's deferred decision(s) 3333, 3346, 3363, 7101 (tmp/backlog/2026-10-02-complete-and-wire/DECISIONS.md)"
subsystem = ["benchmarks/viabilitybench/experiments"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK69"
anchors = ["benchmarks/viabilitybench/analysis/report.py"]
lane = "bench"
parent = "spec-635697"
links = { depends_on = ["gap-625195", "gap-e00238", "gap-f548c1", "gap-cc5051", "gap-cb5133", "gap-de0b87", "gap-057cc7", "gap-5ddf9b", "gap-2ca903", "gap-7c9a9c", "gap-943046", "gap-31c0e8", "gap-c1d920", "gap-85d176", "gap-a42df2", "gap-50346f", "gap-7ec3ef", "gap-0c429f", "gap-4a5109", "gap-147c4d", "gap-940e44", "gap-eb39c1", "gap-2e4a81", "gap-414e56", "gap-ed1a08", "gap-681741"], blocks = [], related = ["gap-d9e9fe"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f benchmarks/viabilitybench/reports/h5_live/metrics.json && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/report.py --check benchmarks/viabilitybench/reports/h5_live"

[[verify]]
command = "test -f benchmarks/viabilitybench/reports/drift/metrics.json && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/report.py --check benchmarks/viabilitybench/reports/drift"

[[verify]]
command = "test -f benchmarks/viabilitybench/reports/p1_live/metrics.json && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/report.py --check benchmarks/viabilitybench/reports/p1_live"

[[verify]]
command = "test -f benchmarks/viabilitybench/reports/p1_ext/metrics.json && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/report.py --check benchmarks/viabilitybench/reports/p1_ext"

[[verify]]
command = "test -f benchmarks/viabilitybench/reports/p1_fd_api/metrics.json && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/report.py --check benchmarks/viabilitybench/reports/p1_fd_api"

[[verify]]
command = "test -f benchmarks/viabilitybench/reports/t5_loo/metrics.json && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/report.py --check benchmarks/viabilitybench/reports/t5_loo"

[[verify]]
command = "test -f benchmarks/viabilitybench/reports/h6_live/metrics.json && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/report.py --check benchmarks/viabilitybench/reports/h6_live"

[[verify]]
command = "grep -qw 'def test_final_report_has_seven_primaries_with_holm_decisions' benchmarks/viabilitybench/analysis/test_final.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_final.py -k test_final_report_has_seven_primaries_with_holm_decisions -q"

[[verify]]
command = "grep -qw 'def test_final_report_has_envelope_pareto_and_frontier_wins' benchmarks/viabilitybench/analysis/test_final.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_final.py -k test_final_report_has_envelope_pareto_and_frontier_wins -q"
+++

## Problem

This package delivers 8 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK69, slice 33xx, phase 8), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3365 | S | p2 | Run E-H5-live: H5-A0, H5-A1 and H5-A3 (BL4) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3365-run-e-h5-live.md` |
| 2 | 3367 | S | p1 | Run the drift anchor (E-drift, BL10) and re-log a drifted arm | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3367-run-drift-anchor-and-relog.md` |
| 3 | 3368 | M | p1 | Run E-P1-live: roko_full against fd_claude and cheap_direct, the confirmatory head-to-head (BL6) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3368-run-e-p1-live-confirmatory-head-to-head.md` |
| 4 | 3369 | M | p2 | Run E-P1-ext on the 60 SWE-bench tasks (BL7) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3369-run-e-p1-ext-swe-bench.md` |
| 5 | 3370 | S | p2 | Run E-fd-api (BL8) and the T5 leave-one-mechanism-out ablation (BL11) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3370-run-e-fd-api-and-t5-ablation.md` |
| 6 | 3364 | S | p2 | Run E-H6-live: H6 Stage B (BL3) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3364-run-e-h6-live-stage-b.md` |
| 7 | 3371 | M | p1 | Final report I: the seven primaries with their Holm decisions, NOT RUN rows included | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3371-final-report-seven-primaries.md` |
| 8 | 3372 | M | p1 | Final report II: envelope table, Pareto data, frontier wins and the external slice | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3372-final-report-envelope-pareto-frontier-wins.md` |

## Why it matters

Phase 8: M1 controller and guarded commit (S06). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3300-the-proof-viabilitybench-pilots-log1-and-head-to-head.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/analysis/envelope_table.py`, `benchmarks/viabilitybench/analysis/report.py`, `benchmarks/viabilitybench/analysis/tab_t6_primaries.py`, `benchmarks/viabilitybench/analysis/test_final.py`, `benchmarks/viabilitybench/reports/drift/`, `benchmarks/viabilitybench/reports/h5_live/`, `benchmarks/viabilitybench/reports/h6_live/`, `benchmarks/viabilitybench/reports/p1_ext/`, `benchmarks/viabilitybench/reports/p1_fd_api/`, `benchmarks/viabilitybench/reports/p1_live/`, `benchmarks/viabilitybench/reports/t5_loo/`.

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

- Waits on: PK01 (gap-625195), PK02 (gap-e00238), PK07 (gap-f548c1), PK09 (gap-cc5051), PK17 (gap-cb5133), PK19 (gap-de0b87), PK26 (gap-057cc7), PK27 (gap-5ddf9b), PK30 (gap-2ca903), PK31 (gap-7c9a9c), PK35 (gap-943046), PK36 (gap-31c0e8), PK43 (gap-c1d920), PK44 (gap-85d176), PK45 (gap-a42df2), PK46 (gap-50346f), PK49 (gap-7ec3ef), PK52 (gap-0c429f), PK57 (gap-4a5109), PK59 (gap-147c4d), PK60 (gap-940e44), PK63 (gap-eb39c1), PK64 (gap-2e4a81), PK66 (gap-414e56), PK67 (gap-ed1a08), PK68 (gap-681741).
- On hold until Will takes the deferred decision(s) 3333, 3346, 3363, 7101 (spend or a public release); see `DECISIONS.md`.
- Existing work items this package covers or touches: gap-d9e9fe. When its tasks are done, close those whose verify then passes.
- Suggested model: sonnet.
