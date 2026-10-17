+++
id = "gap-14b865"
kind = "gap"
title = "roko plan import --mori writes no covers: a step made from an acceptance line never says which criterion it proves, so the plan sheet shows every imported criterion with no check of its own"
status = "open"
triage = "unverified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-cli/plan"]
created = 2026-10-17
updated = 2026-10-17
source = "tmp/hermes-roko-v6 (ui-dash's plan sheet, via team-lead)"
anchors = ["crates/roko-cli/src/plan_import.rs"]
links = { depends_on = [], blocks = [], related = ["gap-624658", "bug-308b5e", "bug-dfe4c5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_import_links_a_checkpoint_step_to_the_criterion_it_proves' crates/roko-cli/ && cargo test -p roko-cli --lib plan_import_links_a_checkpoint_step_to_the_criterion_it_proves"
+++

## Problem

`roko plan import --mori` turns acceptance lines into verify steps but writes no `covers`, so no step says which
criterion it proves. The plan sheet (ui-dash, round v6) then lists every imported criterion with no check of its own
and shows all of the task's steps under "proved by", even when a step was made from one particular line.

A mori task's acceptance mixes three kinds of line:

- lines that become steps:
  - "Checkpoint: \`cmd\` exits 0" lines;
  - lines naming a blocking `verify-tasks.toml` entry by its invariant or its tests;
  - "File exists" lines;
- prose that no step is made from.

Across the bardo corpus's 1,738 tasks:

| Tasks | Count | Of which have a checkpoint |
|---|---|---|
| Some lines become steps and some are prose | 1,156 | about 240 |
| Prose only | 556 | — |
| Every line becomes a step | 25 | — |

## Why it matters

- The plan sheet can show which check proves which criterion only when steps carry `covers`. The mori-import scene
  shows an imported plan as a document.
- PLAN_044 (3208) and SQ03 traceability read `covers` too.

## Where

- `crates/roko-cli/src/plan_import.rs`:
  - `Importer::task_checks`, which makes the steps;
  - `Draft::steps`;
  - `join_cargo_tests`;
  - `push_owned_step`;
  - `render_step`.
- `crates/roko-gate/src/spec_quality.rs::ac_id`, the criterion ids that `covers` names.
- `crates/roko-cli/src/plan_validate.rs::covers_diagnostics` (PLAN_044):
  - an id that names no criterion is an error;
  - in a task that uses `covers`, a criterion that no step covers is a warning, which `--strict` rejects.

## Plan

This is option A from roko-demos' note to the lead on 2026-10-17. The lead had not picked an option when this work
started.

1. A step made from acceptance lines covers them, one to one:
   - a checkpoint step covers its line;
   - a `verify-tasks.toml` step covers the lines that name its invariant or its tests;
   - a `test_fns` step covers the lines that name its tests;
   - the file step covers its "File exists" lines.
2. Prose lines:
   - In a task with a checkpoint step, the checkpoint step also covers the prose lines, since mori accepts a task on
     its checkpoint.
   - A task with no checkpoint keeps `covers` only when every line is covered. Otherwise it writes none, so it gets
     no PLAN_044 warning.
3. When steps are joined, their covers are joined too. A step that moves to the whole-plan check, or that another
   task takes over, drops its covers, because they name its own task's criteria.
4. Tests:
   - `plan_import_links_a_checkpoint_step_to_the_criterion_it_proves`;
   - the corpus test stays at 156 of 156 under `--strict`.

## Done when

- The `[[verify]]` passes.
- The corpus test passes.
- Importing pocket-sequencer gives every T1–T4 criterion a covering step.
