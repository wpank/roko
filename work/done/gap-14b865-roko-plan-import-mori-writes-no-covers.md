+++
id = "gap-14b865"
kind = "gap"
title = "roko plan import --mori writes no covers: a step made from an acceptance line never says which criterion it proves, so the plan sheet shows every imported criterion with no check of its own"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-cli/plan"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "c42c5c8cd"
source = "tmp/hermes-roko-v6 (ui-dash's plan sheet, via team-lead)"
anchors = ["crates/roko-cli/src/plan_import.rs"]
links = { depends_on = [], blocks = [], related = ["gap-624658", "bug-308b5e", "bug-dfe4c5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_import_links_a_checkpoint_step_to_the_criterion_it_proves' crates/roko-cli/ && cargo test -p roko-cli --lib plan_import_links_a_checkpoint_step_to_the_criterion_it_proves"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T20:39:33Z"
commit = "c42c5c8cd"
forced = false
evidence = "Option B (the lead, 2026-10-17): roko plan import --mori keeps in acceptance only the lines a step checks, each step covers the lines it checks (a checkpoint its own line and those whose named tests its suite defines; a verify-tasks or test_fns step those whose named tests it runs, else its invariant; the file step its File exists lines; never a line naming a test it does not run), and the other lines move verbatim to the end of the description under Also required, not checked automatically: (plus (the review task judges these) with a reviewer); lines past the description room go to mori-also-required-<task>.txt, which the task reads; the report counts each split. Live, zero cost, pocket-sequencer: before, 16 acceptance lines with no covers; after, T1 4 checked + 2 also required, T2 3 + 2, T3 3 + 2, T4 3 + 1, every checked line covered by the steps that run its tests (INV-11 by the checkpoint suite, not by the step that runs only test_cli), 0 diagnostics under --strict. On c42c5c8cd (main 4bae170bb plus the change): the verify, the 6 importer tests, the mori corpus test (156 of 156 under --strict), clippy -D warnings (roko-cli) and nightly fmt pass."
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

The lead chose option B on 2026-10-17. Option A, where a checkpoint covers the prose lines, was dropped: it would
show prose criteria as covered by a checkpoint that does not check them, which overstates verification.

1. Each step covers the lines it checks:
   - A checkpoint step covers its own line, plus every line whose named tests (`test_…` words) its suite defines.
     The suite is the file a `python3 -m unittest <module>` names, or the crate a `cargo test -p <package>` with no
     filter builds.
   - A `verify-tasks.toml` step or a `test_fns` step covers the lines whose named tests it runs. A line naming no
     test is covered by the step whose entry's invariant the line names.
   - The steps that run every test a line names cover it. Failing that, the steps that run those tests between them
     cover it. A line that names a test no step runs stays unchecked.
   - The file step covers its "File exists" lines.
2. `acceptance` keeps only the checked lines, renumbered, so `covers` stays one to one.
3. The other lines move verbatim to the end of the description:
   - They go in the block `Also required, not checked automatically:`, with the line
     `(the review task judges these)` when the plan has a review task, then one `- ` line each.
   - The tier fit counts the block. The lead is cut to the room the block leaves, but to no fewer than 60 words.
   - Block lines past the description's room go to `mori-also-required-<task>.txt` beside the plan. The task reads
     that file, and a closing sentence names it.
4. Steps that are joined join their covers. A step that moves to the whole-plan check, or that another task takes
   over, drops its covers.
5. The import report counts each task's split, for example `acceptance: T1: 4 checked, 2 also required`.
6. Tests:
   - `plan_import_links_a_checkpoint_step_to_the_criterion_it_proves`;
   - the corpus test stays at 156 of 156 under `--strict`.

## Done when

- The `[[verify]]` passes.
- The corpus test passes, with 156 of 156 plans clean under `--strict`.
- Importing pocket-sequencer:
  - covers each of T1–T4's checked lines with the steps that run their tests;
  - lists the rest in the block;
  - has 0 diagnostics under `--strict`.
