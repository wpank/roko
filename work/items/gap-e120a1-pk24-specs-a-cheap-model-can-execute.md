+++
id = "gap-e120a1"
kind = "gap"
title = "PK24 Specs a cheap model can execute: Refiner loop against a stub model: additive only, with sources (+3 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
rank = 24
size = "M"
subsystem = ["benchmarks/viabilitybench/specops"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK24"
anchors = ["benchmarks/viabilitybench/requirements.in", "benchmarks/viabilitybench/speclint/speclint.py"]
lane = "bench"
parent = "spec-fef7c5"
links = { depends_on = ["gap-cb5133", "gap-de0b87", "gap-1149aa", "gap-46fd19", "gap-eb1aa3"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_refiner_never_deletes_or_weakens_a_verify_step' benchmarks/viabilitybench/specops/tests/test_refine.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/specops/tests/test_refine.py -q"

[[verify]]
command = "grep -qw 'def test_default_run_makes_no_network_call' benchmarks/viabilitybench/specops/tests/test_critic.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/specops/tests/test_critic.py -q"

[[verify]]
command = "grep -q 'within the $0.02 cap: yes' benchmarks/viabilitybench/reports/refiner-cost/summary.md"

[[verify]]
command = "grep -qw 'def test_vague_variants_score_30_below_precise' benchmarks/viabilitybench/specops/tests/test_manipulation.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/specops/tests/test_manipulation.py -q"
+++

## Problem

This package delivers 4 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK24, slice 32xx, phase 3), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3235 | M | p3 | Refiner loop against a stub model: additive only, with sources | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3235-refiner-loop-additive-with-sources.md` |
| 2 | 3237 | S | p3 | Optional critic and ambiguity probe, off by default | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3237-optional-critic-and-ambiguity-probe.md` |
| 3 | 3236 | S | p3 | Live refiner cost sample: 20 tasks at no more than $0.02 each | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3236-live-refiner-cost-sample.md` |
| 4 | 3234 | S | p2 | Manipulation check: vague variants score 30 below their precise twins | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3234-manipulation-check-vague-30-below-precise.md` |

## Why it matters

Phase 3: golden-path proof. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3200-specs-a-cheap-model-can-execute.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/reports/refiner-cost/summary.md`, `benchmarks/viabilitybench/requirements.in`, `benchmarks/viabilitybench/speclint/speclint.py`, `benchmarks/viabilitybench/specops/ambiguity.py`, `benchmarks/viabilitybench/specops/critic.py`, `benchmarks/viabilitybench/specops/manipulation_check.py`, `benchmarks/viabilitybench/specops/refine.py`, `benchmarks/viabilitybench/specops/tests/test_critic.py`, `benchmarks/viabilitybench/specops/tests/test_manipulation.py`, `benchmarks/viabilitybench/specops/tests/test_refine.py`.

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

- Waits on: PK17 (gap-cb5133), PK19 (gap-de0b87), PK21 (gap-1149aa), PK22 (gap-46fd19), PK23 (gap-eb1aa3).
- Suggested model: sonnet.
