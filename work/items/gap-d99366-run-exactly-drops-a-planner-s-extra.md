+++
id = "gap-d99366"
kind = "gap"
title = "run_exactly drops a planner's extra test chained after a spec command (tools/spec.sh 2 lfo 'P2.01*' && ctest -R extra keeps only the spec command) instead of moving it to a step of its own"
status = "open"
triage = "verified"
severity = "p3"
size = "S"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
source = "review:roko-planner-2 of bug-56a806 (e87ec04e3)"
discovered_from = "agent:roko-planner-2 (round v7 review)"
anchors = ["crates/roko-cli/src/plan_generate/source_checks.rs::run_exactly"]
links = { depends_on = [], blocks = [], related = ["bug-56a806"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn a_planners_extra_test_gets_its_own_step' crates/roko-cli/src/plan_generate/source_checks.rs && cargo test -p roko-cli --lib -- a_planners_extra_test_gets_its_own_step"
+++

## Problem

`source_checks::run_exactly` (bug-56a806) cuts a step that runs a source check's command down to the source's
commands. A test of the planner's own chained after it is dropped with the rest, not moved to a step of its own:
`tools/spec.sh 2 lfo 'P2.01*' && ctest -R extra` becomes `tools/spec.sh 2 lfo 'P2.01*'`, and `ctest -R extra` runs
nowhere. Coverage is right (the source's check is kept); a check the planner meant is lost without a word beyond the
repair line. Found by roko-planner-2 reviewing e87ec04e3.

## Why it matters

Low: the source's checks are what decide a task, and the repair line names what was dropped. But a planner that
adds a real test for a criterion the source does not cover loses it.

## Where

- `crates/roko-cli/src/plan_generate/source_checks.rs::run_exactly` and its helper `exact_steps`, run by
  `plan_generate/pipeline.rs::generate_plan` after `assign_runs`.

## Current state

Every command around the source's commands goes: guards (`grep -q`, `test -f`), builds and other runners alike.

## Plan

When the task is under its tier's step limit (`max_verify_steps`, as `assign_runs` gets it), move the dropped
commands that run something (not a grep, a file test or a build, see `is_shallow`) into a new step after this one,
with the same `covers` that are not the source's; otherwise drop them as now. Never keep a guard before a source
command.

## Done when

- `tools/spec.sh 2 lfo 'P2.01*' && ctest -R extra` in a task under its limit becomes two steps,
  `tools/spec.sh 2 lfo 'P2.01*'` and `ctest -R extra`; at the limit it stays one step, the source's command alone.
- The `[[verify]]` test passes.

## Notes

- The lead's call (round v7, 2026-10-20): file it, don't build it now.
