+++
id = "bug-4e89b8"
kind = "bug"
title = "A task that leaves a file its spec says to delete passes its gate: no check covers a deletion, so phase 1's T2 kept the nine plugin/ui files its spec retires and the run passed"
status = "open"
triage = "verified"
severity = "p1"
goal = "core"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch", "roko-cli/plan_generate"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "daf36c3ab"
source = "round v8: the lead, from phase 1's run on roko ba5866664 (plan op 541860eb, Undertow 62155c4)"
discovered_from = "agent:team-lead (round v8)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs::attempt_verify_steps", "crates/roko-cli/src/graph_task_dispatch/verification.rs::verify_steps", "crates/roko-cli/src/plan_generate/named_files.rs::named_files"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-aa93e9", "gap-708931"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_task_that_leaves_a_file_its_spec_deletes_fails_its_gate' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_task_that_leaves_a_file_its_spec_deletes_fails_its_gate"

[[verify]]
command = "grep -rqw 'fn the_files_a_task_deletes_come_from_its_spec' crates/roko-cli/src/ && cargo test -p roko-cli --lib the_files_a_task_deletes_come_from_its_spec"
+++

## Problem

A task whose spec says to delete files can pass its gate without deleting them. No check covers a deletion.

Round v8's phase 1 run on roko ba5866664 (the plan from op 541860eb, Undertow 62155c4) passed 3/3 on first tries:
- T2's `files` held the nine `plugin/ui/` files its spec retires ("Deletes: `plugin/ui/Displays.h`, …"), which
  bug-aa93e9 put there.
- T2's agent deleted none of them.
- Its checks (`tools/spec.sh 1 plugin`, `tools/pluginval.sh 5`) pass with the files still there, so the run passed and
  merged them.

## Why it matters

- An unchecked deletion doesn't get done. Every later phase that retires files depends on it, and a dead file left in
  the tree misleads the next phase's agent and the person reading the code.
- The lead's ask, 2026-11-12: the task's gate checks that those paths are gone, and fails in plain words ("task T2
  should have deleted plugin/ui/Look.cpp"), so the next try fixes it. Explicit spec-named deletions only.
- Goal: core.

## Where

- `crates/roko-cli/src/graph_task_dispatch/verification.rs`: `verify_steps` and `attempt_verify_steps`, the steps
  every attempt's gate runs (and its prompt lists).
- `crates/roko-cli/src/plan_generate/named_files.rs` (bug-aa93e9): the reading of "Delete:", "Deletes:" and "Delete
  these nine:" sentences.

## Plan

1. `named_files::files_task_deletes(plan, task, workdir)`. These are the task's `files` that a sentence of the plan's
   `[meta] sources` tells a task to delete. It uses bug-aa93e9's reading, but whether or not the file is still there,
   since a try may have deleted it already.
2. `verify_steps` puts a step labelled `rung[deleted-files]` first, ahead of the authored steps and the workspace
   rungs. It is a short shell check that prints "task T2 should have deleted plugin/ui/Look.cpp" for each such file
   still present, and fails. It costs no build, and the try's feedback and the dashboard carry its lines like any
   step's. The prompt lists it among the task's checks, so the agent sees which files to delete.
3. Tests, red first:
   - `a_task_that_leaves_a_file_its_spec_deletes_fails_its_gate`: a dispatcher fixture whose plan's spec says
     "Deletes: `src/old.rs`". The attempt that leaves it fails, naming it. Once it's gone, the gate passes.
   - `the_files_a_task_deletes_come_from_its_spec`: on phase 1's spec and repaired plan, T1 deletes its 12 files and
     T2 its 9.

## Done when

- Both `[[verify]]` commands pass.
- A plan with no `[meta] sources`, or no named deletion, gets no new step.

## Notes

- Found by the lead from phase 1's run (2026-11-12).
- Related: bug-aa93e9 (named files in `files`), gap-708931 (deletions in words get a warning, not this gate).
