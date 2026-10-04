+++
id = "gap-b10978"
kind = "gap"
title = "Self-model replay matrix keys cells by (task, arm) only, collapsing arms that ran several models into one"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-learn/self-model", "benchmarks/viabilitybench/analysis"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-8 follow-up reports 2026-10-03 (PK49 gap-7ec3ef)"
discovered_from = "gap-7ec3ef"
anchors = ["crates/roko-learn/src/self_model/replay.rs::Matrix"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn matrix_keeps_one_cell_per_task_arm_and_model' crates/roko-learn/ && cargo test -p roko-learn matrix_keeps_one_cell_per_task_arm_and_model"
+++

## Problem

The M3 self-model replay's matrix (`crates/roko-learn/src/self_model/replay.rs::Matrix`) keys each cell by
`(task, arm)` only (`cells: BTreeMap<(String, String), Cell>`, line ~95), with `Cell.model_requested` carried as
plain metadata, not part of the key. Its own doc comment: "Each (task, arm) keeps its lowest seed" — when
multiple runs exist for the same (task, arm) with *different models* (e.g. `roko_fixed` on LOG1 block A, which
S09's spec runs "× 4 cheap models × P1-core × 2 seeds," `tmp/cybernetic-harness/specs/S09-experiments.md:229`;
or `cheap_direct`, which can also vary its served model per run), only one `Cell` survives per (task, arm) — the
others are silently discarded, with no per-model distinction at all.

`benchmarks/viabilitybench/analysis/econ.py::_matrix` (line ~523-530) does the identical join:
`key = (record["task"]["instance_id"], record["arm"])` — the same collapse, in Python, confirming "econ.py
copies that join."

## Why it matters

Goal: cybernetic, M3/S09. An arm that actually ran several different models (by design, per S09's LOG1 block A)
gets measured as if it were one model — whichever run happened to be kept per (task, arm) — silently discarding
the others' outcomes. Any report built from this matrix (the M3 replay's baselines, econ.py's cost/VS-rate
tables) under-counts real variance across models within an arm and can attribute one model's result to the arm
as a whole.

## Where

- `crates/roko-learn/src/self_model/replay.rs::Matrix`, `Matrix::from_records`, `Cell`.
- `benchmarks/viabilitybench/analysis/econ.py::_matrix`.

## Current state

Confirmed as described on both the Rust and Python sides; unaddressed.

## Plan

1. Add the model (or arm key, if that's the more natural unit) to the matrix's key on both sides, so each
   (task, arm, model) combination gets its own cell.
2. Decide how downstream consumers (replay baselines, econ.py's tables) should aggregate across a multi-model
   arm's cells when a per-arm (not per-model) summary is wanted — mean, worst-case, or keep them separate and let
   the report show the spread.
3. Add a regression test/fixture with one (task, arm) pair recorded under two different models, asserting both
   survive as distinct cells.

## Done when

- A (task, arm) pair recorded under multiple models produces one matrix cell per model on both the Rust and
  Python sides, not one collapsed cell.
- The `[[verify]]` command passes.

## Notes

- Discovered during PK49's M3 self-model work (gap-7ec3ef, done).

## Progress

- gap-b10978: implemented at a3123051f on `work/bug-78e5ce`; cargo verification deferred to the batch gate.
  `Matrix::from_records` keeps one cell per (task, arm, model): an arm whose runs asked for more than one model
  becomes one arm per model, labelled `<arm>[<model>]` (`replay::arm_label`, `<arm>[?]` for a run with no
  model). An arm with one model keeps its name, so existing fixtures and policies are unchanged.
  `Matrix::arm_models` replaces the CLI's own file re-read, so split arms keep their model in
  `roko learn self-model replay`. econ.py's `_matrix` labels the same way and keeps only VS-labelled runs, as
  Rust does, so trace rows join. Aggregation decision: the replay keeps each (arm, model) separate as its own
  routing arm; econ.py's per-arm tables still pool an arm's runs across its models. Tests:
  `matrix_keeps_one_cell_per_task_arm_and_model` (Rust, `replay.rs`) and its Python namesake in `test_econ.py`.
  The analysis suite passes in a worktree venv: 101 passed, 1 skipped.
