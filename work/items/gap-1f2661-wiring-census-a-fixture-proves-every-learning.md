+++
id = "gap-1f2661"
kind = "gap"
title = "Wiring census: a fixture proves every learning loop reads the settled attempt record (S01.P0-11, S01.P0-12)"
status = "open"
triage = "unverified"
severity = "p1"
goal = "truth"
size = "M"
subsystem = ["roko-cli/tests", "roko-cli/graph_execution"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e4"
discovered_from = "tmp/cybernetic-harness/specs/S01-instrumentation.md (P0-11, P0-12, §4.8, §5.8, §7)"
anchors = ["crates/roko-cli/tests/learning_wiring_census.rs", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan_body"]
lane = "rust-cold"
parent = "spec-b7303f"
links = { depends_on = ["gap-96f7ed"], blocks = [], related = ["gap-cd3529"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn loop_census_fixture_settles_one_record_per_attempt' crates/roko-cli/tests/ && grep -rqw 'fn graph_dispatcher_production_wiring_census' crates/roko-cli/tests/ && cargo test -p roko-cli --test learning_wiring_census"
+++

## Problem

No test shows that the learning loops are attached to the production object graph, or that they read the settled
attempt record. `crates/roko-cli/tests/phase0_wiring.rs` holds six smoke tests, such as "router persists" and
"episodes logged", but it is not a census. A sink can drop off the Graph feedback facade, and nothing fails.

## Why it matters

This test is the exit check for epic spec-b7303f. S01 calls the census "the gate that every loop must pass before any
experiment uses it". S03 (M2) and S09 refuse any loop that it does not show as live.

## Where

- **New file:** `crates/roko-cli/tests/learning_wiring_census.rs`.
- **Two seams:**
  - `GraphTaskDispatcher::wiring_report()`, a small accessor;
  - `build_graph_feedback_facade(..)`, extracted from the sink block in `run_graph_plan_body` (`plan_runner.rs`,
    about lines 938–1010). That block registers the episode, routing, dream, daimon, theta and delta sinks.
- **Pattern for the fixture run:** `tests/graph_budget_resume.rs`.

## Current state

Nothing exists at `41c7ffbd6`. bug-0b668a, which isolates `.roko/learn` in tests, is done.

## Plan

1. **Extract the two seams** with no change in behaviour.
2. **Write `graph_dispatcher_production_wiring_census`.** Every component in S01 §5.8 must be either wired or listed
   in `EXPECTED_MISSING`, and a newly missing component fails the test. Today's `EXPECTED_MISSING`:
   - knowledge ingestion;
   - section writer;
   - prompt-experiment context;
   - decision writer and exposure writer;
   - `record_access`.
3. **Write `loop_census_fixture_settles_one_record_per_attempt`.** It runs four tasks with a scripted fake provider:
   - T1 passes its verify step;
   - T2 fails its verify step with `max_retries = 1`;
   - T3 has no verify step;
   - T4 pins an unconfigured model.

   From the files alone, assert:
   - 5 `attempt_open` lines, and 5 verdicts that each carry an attempt key, a verdict, the executed model and a cost
     source;
   - T3's label is null, and T3 leaves the router state unchanged;
   - every episode, efficiency and cost row joins to an attempt key.

## Done when

- [ ] Both tests exist and pass.
- [ ] Removing the routing sink from the facade makes the census fail. Check this once by hand.
- [ ] The `[[verify]]` command passes.

## Notes

The seams add a few lines each to two hot files, `graph_task_dispatch.rs` and `plan_runner.rs`. Land them in the same
window as gap-96f7ed.
