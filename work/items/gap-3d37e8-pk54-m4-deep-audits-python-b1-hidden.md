+++
id = "gap-3d37e8"
kind = "gap"
title = "PK54 M4 deep audits: Python: B1 hidden suites written from the spec by another model family, and validated…"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 54
size = "M"
hold = "waits on Will's deferred decision(s) 7101 (tmp/backlog/2026-10-02-complete-and-wire/DECISIONS.md)"
subsystem = ["benchmarks/viabilitybench/audit"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK54"
anchors = ["benchmarks/viabilitybench"]
lane = "bench"
parent = "spec-c3abc8"
links = { depends_on = ["gap-dff960"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_b1_suite_is_rejected_unless_it_fails_on_base' benchmarks/viabilitybench/audit/tests/test_b1.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/audit/tests/test_b1.py -q"
+++

## Problem

This package delivers 1 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK54, slice 71xx, phase 7), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 7109 | M | p2 | Python: B1 hidden suites written from the spec by another model family, and validated before use | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7109-python-b1-suites-from-another-family.md` |

## Why it matters

Phase 7: M4 random deep audits (S05). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7100-m4-random-deep-audits.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/audit/b1_author.py`, `benchmarks/viabilitybench/audit/battery.py`, `benchmarks/viabilitybench/audit/tests/test_b1.py`.

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

- Waits on: PK53 (gap-dff960).
- On hold until Will takes the deferred decision(s) 7101 (spend or a public release); see `DECISIONS.md`.
- Suggested model: opus.
