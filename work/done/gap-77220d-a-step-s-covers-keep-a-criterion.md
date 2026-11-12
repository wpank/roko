+++
id = "gap-77220d"
kind = "gap"
title = "A step's covers keep a criterion twice when two source checks map to it (phase 5's T1: [\"AC1\", \"AC1\"]), and assign_runs joins a task's missing checks into one && step even with room for more, against rule 3"
status = "done"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "09a8f162e"
source = "round-v7-roko-planner#phase-5-replay"
discovered_from = "agent:roko-planner (round v7, phase 5 $0 replay on roko 4d22b94e8)"
anchors = ["crates/roko-cli/src/plan_generate/source_checks.rs::normalize_covers", "crates/roko-cli/src/plan_generate/source_checks.rs::assign_runs"]
links = { depends_on = [], blocks = [], related = ["bug-72dc0d", "bug-59782c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn missing_checks_get_steps_of_their_own_and_covers_stay_unique' crates/roko-cli/src && cargo test -p roko-cli --lib -- missing_checks_get_steps_of_their_own_and_covers_stay_unique"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T21:09:47Z"
commit = "b3c2b4746"
by = "roko-planner (round v7)"
executor = "claude-agent"
via = "manual"
forced = false
evidence = "Merged b3c2b4746 (branch 0522d8de1; main merged in at 9b9db9caa). Post-merge verify in main at b3c2b4746: the grep finds the test and missing_checks_get_steps_of_their_own_and_covers_stay_unique passes, with a_summary_naming_its_spec_is_planned_with_the_specs_checks (2 of 2). Branch: plan_generate and plan_policy 194/194; roko-cli lib 4042 passed, 2 failed (graph_execution::plan_runner::tests::frozen_learning_*, failing on main since 754f45ebd); nightly fmt; clippy -p roko-cli -D warnings clean. normalize_covers keeps each criterion once per step (phase 5's T1 and T8 had AC1 two and three times); assign_runs gives each missing command a step of its own while the task has room, each with its own covers and expect, the rest joining the last one at the limit."
+++

## Problem

Two small defects in roko's repairs of a draft's source checks, seen in the phase-5 $0 replay
(`tmp/hermes-roko-v7/roko-planner/replay-p5/`, Undertow `spec/p5-f15`, roko 4d22b94e8):

1. **Duplicate covers.** A step whose `covers` name two source checks that one criterion states keeps the criterion
   twice. Phase 5's T1 runs `tools/spec.sh 5 presets` for P5.01-P5.03 (SC1) and P5.09 (SC7). The repairs say "T1:
   `covers` SC1 became AC1" and "T1: `covers` SC7 became AC1", and the step ends with `covers = ["AC1", "AC1"]`
   (`source_checks::normalize_covers`).
2. **Missing checks joined with `&&`.** When a task's draft runs none of its regression checks, `assign_runs` adds
   them as one step: "T1: added a step running `tools/spec.sh 2_2 filter && tools/spec.sh 2_1 filter`, the source's
   own commands for SC9, SC10" (the replay's one-task probe draft). Rule 3 of the generator prompt says the steps of
   different criteria stay separate, never chained with `&&`, because the first failure of a chain hides the rest.
   bug-72dc0d's "a task's checks run as one step" joins any of a task's missing checks the same way, also when the
   task has room for more steps.

## Why it matters

1. `plan validate` reports nothing, but a per-criterion count reads AC1 twice.
2. A failing first command hides the second, so a retry sees one failure where there may be two. A task missing both
   a fail-on-base check and a regression check gets one step with no `expect`.

## Where

- `crates/roko-cli/src/plan_generate/source_checks.rs`: `normalize_covers` and `assign_runs`.

## Plan

1. `normalize_covers`: keep each criterion once in a step's `covers`.
2. `assign_runs`: give each missing check, or each group of checks with one command, a step of its own while the
   task has room under its tier's step limit. Join into a step only at the limit, as now. A step of regression
   checks alone keeps `expect = "pass_on_base"`.

## Done when

A test pins both: the covers stay unique, and two missing regression checks become two steps when the task has room.
