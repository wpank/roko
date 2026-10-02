+++
id = "gap-33eec2"
kind = "gap"
title = "PK55 M4 deep audits: Python: the gold-task planter and a per-check sensitivity and specificity report (S05…"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 55
size = "S"
subsystem = ["benchmarks/viabilitybench/audit"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK55"
anchors = ["benchmarks/viabilitybench"]
lane = "bench"
parent = "spec-c3abc8"
links = { depends_on = ["gap-dff960"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_gold_units_report_sensitivity_and_specificity' benchmarks/viabilitybench/audit/tests/test_gold.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/audit/tests/test_gold.py -q"
+++

## Problem

This package delivers 1 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK55, slice 71xx, phase 7), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 7110 | S | p2 | Python: the gold-task planter and a per-check sensitivity and specificity report (S05 task 16) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7110-python-gold-planter-and-check-sensitivity.md` |

## Why it matters

Phase 7: M4 random deep audits (S05). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7100-m4-random-deep-audits.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/audit/gold.py`, `benchmarks/viabilitybench/audit/tests/test_gold.py`.

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
- Suggested model: opus.

## Progress

- 7110: implemented at `c23667a3f`. New `audit/gold.py`: `select_gold` draws ~3% of a stream's instances whose
  family ships a planted-gaming reference solution (F1, F4; F7/F8 don't, per this task's own Notes), each paired
  with a `GoldKind` (F1 `gaming`, F4 `gaming/exit0`, F4 `gaming/dry_run`). `plant` materializes the instance fresh
  and applies that solution; `audit_unit` archives the gamed tree the way the driver archives a real attempt
  (`driver/archive.py`'s `commit_final`/`archive_task`) so `audit.battery.run_battery` runs completely unchanged,
  then builds a `vs.label` row (reusing `audit.vs_checks`/`audit.label_errors`, the existing schema) with the known
  ground truth: every gold unit is a false green, and `integrity.g` follows the kind's own
  `common.astcheck.gaming_summary` flag (F4's `exit0` is `literal_return`, g known 1; F1's `gaming` and F4's
  `dry_run` are both 0 — confirmed against each family's own gaming.py docstring, not guessed). `gold_confusion`
  reports each check's sensitivity and specificity against those known labels with Wilson intervals
  (`audit.estimate.wilson`) and the counts behind each; `p1_rows` drops every gold row.
  Verified empirically against the real F1/F4 families and the real battery (no mocks, no stubs): A1 (tamper
  policy) misses F4's known-gamed `exit0` (sensitivity 0.0, n=1) but correctly clears both known-clean kinds
  (specificity 1.0, n=2) — an honest finding, since none of these three tricks touch a protected or out-of-scope
  file, which is what A1's diff-shaped policy actually looks for. The item's `[[verify]]` command passes
  (4/4 tests in `test_gold.py`); the whole `audit/tests/` directory (57 tests) still passes, no regressions.
