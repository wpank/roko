+++
id = "bug-308b5e"
kind = "bug"
title = "Imported mori plans warn PLAN_051 under --strict: verify-tasks steps keep mori's phase names (invariant), so the corpus fell from 156 to 127 of 156 strict-clean"
status = "open"
triage = "unverified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-cli/plan"]
created = 2026-10-17
updated = 2026-10-17
source = "roko-demos, bug-dfe4c5's corpus run after gap-e2d2e5 (PLAN_051)"
anchors = ["crates/roko-cli/src/plan_import.rs"]
links = { depends_on = [], blocks = [], related = ["gap-e2d2e5", "gap-b1f7b9", "gap-624658"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_import_names_test_runs_with_a_phase_gate_learning_observes' crates/roko-cli/ && cargo test -p roko-cli --lib plan_import_names_test_runs_with_a_phase_gate_learning_observes"
+++

## Problem

gap-e2d2e5 added PLAN_051, a warning for a plan where no verify step has a phase that gate learning observes
(`roko_gate::rung_for_verify_phase`) even though some step runs tests or compiles. Imported mori plans keep mori's own
phase names on many of their steps:

- a task's `verify-tasks.toml` entries take the entry's `type`, for example `invariant`;
- gap-b1f7b9 renamed only the checkpoint and task-command steps that run tests (`check_phase`).

So the mori corpus import (`plan_import_mori_corpus_validates`) fell from 156 of 156 to 127 of 156 under
`plan validate --strict`: 29 plans warn PLAN_051. All 156 still validate without `--strict`, and the test's floors
(132 and 19) still pass, so nothing failed.

## Why it matters

- An imported plan should be clean under `--strict`, as it was when gap-624658 landed.
- A run of such a plan moves no gate threshold, because its test runs are filed under `invariant`.
- The lead's request for `covers` in imports (U-covers) also asks that the corpus stay at 156 of 156 under `--strict`.

## Where

- `crates/roko-cli/src/plan_import.rs`:
  - `Draft::add_check`, through which every task-level check passes;
  - `check_phase` (gap-b1f7b9), which covers two call sites only.
- `crates/roko-cli/src/plan_validate.rs::gate_learning_message` (PLAN_051).

## Plan

1. `Draft::add_check` gives every task-level check an observed phase when its own phase is not one:
   - `test` when it runs tests;
   - `clippy` when it lints;
   - `compile` when it compiles;
   - otherwise its phase stays.

   `check_phase` goes, since `add_check` now covers its two call sites. Whole-plan steps keep their `verify.sh` and
   `rubric` phases, which PLAN_051 does not read.
2. The corpus test asserts that every plan passes under `--strict`.
3. Test `plan_import_names_test_runs_with_a_phase_gate_learning_observes`.

## Done when

- The `[[verify]]` passes.
- The corpus import gives 156 of 156 under `--strict` again.
