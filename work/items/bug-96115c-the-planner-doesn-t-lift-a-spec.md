+++
id = "bug-96115c"
kind = "bug"
title = "The planner doesn't lift a spec's named final check ('tools/check.sh is the plan's final check') into [[meta.verify]], and the spec-check report can't notice it is missing"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "round-v7-hermes-pilot#D-p1-plan"
discovered_from = "agent:hermes-pilot (round v7 phase 1, op f8dc4c50)"
anchors = ["crates/roko-cli/src/plan_generate/source_checks.rs::source_checks", "crates/roko-cli/src/plan_generate.rs::build_generator_system_prompt"]
links = { depends_on = [], blocks = [], related = ["bug-3cdc58", "bug-daa54e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_specs_named_final_check_becomes_meta_verify' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_specs_named_final_check_becomes_meta_verify"
+++

## Problem

Round v7, Undertow phase 1 (roko f33fd217c, planner gpt-5-4-mini, plan `p1-undertow-voice`). The spec's notes for the
planner say:

> `tools/check.sh` (every present spec test, phases 0 and 1) is the plan's final check, not any one task's.

The plan has no `[[meta.verify]]` at all. Each task's own checks run only that task's spec tests. Nothing in the run
checks phase 0 and phase 1 together (`tools/check.sh` runs every present spec test, phase 0's included) before the
run delivers. Phase 0's plan (same spec style) had the same gap. Its P0.15 was a task check there, so it was covered
by accident.

## Why it matters

A plan-level check is how a phase proves it didn't break the phase before it. The person stated it in the spec;
the planner dropped it without saying so. The spec-check report (bug-3cdc58 aside) counts only the table's checks,
so it can't notice that a named final check is missing.

## Where

- `crates/roko-cli/src/plan_generate.rs`: the generator prompt's `[[meta.verify]]` guidance.
- `crates/roko-cli/src/plan_generate/source_checks.rs`: the source checks are lifted from the checks table and
  lists. A sentence naming a command as "the plan's final check" isn't one of them.

## Plan

1. Lift a command that the source names as the plan's final or whole-plan check ("is the plan's final check", "run
   at the end", "the plan's check") as a source check whose target is `[meta.verify]`, not a task.
2. The coverage report then counts it like any other: kept when `[[meta.verify]]` runs it, otherwise missing, and
   asked again.
3. Prompt: a spec-named final check goes in `[[meta.verify]]` with its command unchanged.

## Done when

Generated from phase 1's spec, the plan has `[[meta.verify]] command = "tools/check.sh"`, or the report lists it as
missing. Checked by the test in `[[verify]]`.

## Notes

- Evidence: `~/dev/undertow/plans/p1-undertow-voice/tasks.toml` (no `[[meta.verify]]`); the spec is
  `~/dev/undertow/docs/specs/phase-1-voice.md`.
- Related:
  - bug-3cdc58: coverage by command;
  - bug-daa54e: shared files left out of a task's files.
