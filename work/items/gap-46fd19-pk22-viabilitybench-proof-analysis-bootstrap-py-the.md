+++
id = "gap-46fd19"
kind = "gap"
title = "PK22 ViabilityBench proof: analysis/bootstrap.py: the paired bootstrap stratified by family and level (+5 more)"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
rank = 22
size = "L"
subsystem = ["benchmarks/viabilitybench/analysis"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK22"
anchors = ["benchmarks/viabilitybench/analysis", "benchmarks/viabilitybench/families"]
lane = "bench"
parent = "spec-fef7c5"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_stratified_bootstrap_recovers_a_planted_difference' benchmarks/viabilitybench/analysis/test_bootstrap.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_bootstrap.py -k test_stratified_bootstrap_recovers_a_planted_difference -q"

[[verify]]
command = "test -f benchmarks/viabilitybench/families/f2_apipager/hidden.py && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/ci/verify_verifiers.py --families f2 --levels 1-5 --seeds 2"

[[verify]]
command = "test -f benchmarks/viabilitybench/families/f3_moneyround/hidden.py && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/ci/verify_verifiers.py --families f3 --levels 1-5 --seeds 2"

[[verify]]
command = "test -f benchmarks/viabilitybench/families/f5_sqlmigrate/hidden.py && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/ci/verify_verifiers.py --families f5 --levels 1-5 --seeds 2"

[[verify]]
command = "test -f benchmarks/viabilitybench/families/f7_rustiter/hidden.py && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/ci/verify_verifiers.py --families f7 --levels 1 --seeds 1"

[[verify]]
command = "test -f benchmarks/viabilitybench/families/f8_honeypot/hidden.py && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/ci/verify_verifiers.py --families f8 --levels 1-5 --seeds 2"
+++

## Problem

This package delivers 6 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK22, slice 33xx, phase 3), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3320 | M | p1 | analysis/bootstrap.py: the paired bootstrap stratified by family and level | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3320-analysis-paired-stratified-bootstrap.md` |
| 2 | 3321 | M | p2 | Family F2 api-pager: generator, truth suite, gaming detector and ladder | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3321-family-f2-apipager.md` |
| 3 | 3322 | M | p2 | Family F3 money-rounding: generator, differential truth suite and ladder | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3322-family-f3-moneyround.md` |
| 4 | 3323 | M | p2 | Family F5 sql-migrate: generator, migration truth suite and ladder | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3323-family-f5-sqlmigrate.md` |
| 5 | 3324 | M | p2 | Family F7 rust-iter: generated crates built in their own CARGO_TARGET_DIR | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3324-family-f7-rustiter.md` |
| 6 | 3325 | M | p2 | Family F8 honeypot-conflict: a wrapper over F1-F5 that plants a contradicting assertion | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3325-family-f8-honeypot-conflict-wrapper.md` |

## Why it matters

Phase 3: golden-path proof. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3300-the-proof-viabilitybench-pilots-log1-and-head-to-head.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/analysis/bootstrap.py`, `benchmarks/viabilitybench/analysis/test_bootstrap.py`, `benchmarks/viabilitybench/families/f2_apipager/`, `benchmarks/viabilitybench/families/f3_moneyround/`, `benchmarks/viabilitybench/families/f5_sqlmigrate/`, `benchmarks/viabilitybench/families/f7_rustiter/`, `benchmarks/viabilitybench/families/f8_honeypot/`.

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
- Suggested model: sonnet.
