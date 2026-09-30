+++
id = "bug-9ab6b8"
kind = "bug"
title = "observe_multi_objective_outcome never advances stage_tracking or refreshes the Pareto frontier"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/cascade_router"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "a8159e1ec"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-router2's report, branch work/bug-f68404 at 732728ee8)"
anchors = ["crates/roko-learn/src/cascade_router.rs"]
lane = "rust-hot"
parent = "spec-6ac537"
links = { depends_on = [], blocks = [], related = ["bug-f68404", "bug-605a8a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn multi_objective_observations_advance_the_stage_and_the_frontier' crates/roko-learn/src/ && cargo test -p roko-learn --lib multi_objective_observations_advance_the_stage_and_the_frontier"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in a5f1d160f. Batch 12a gate on the merged tree (MAIN a8159e1ec has the same tree as gated 265acb18b): cargo check --workspace --tests, nightly fmt --check (after the coordinator's rustfmt commits a3509fc46 and 6144df24b) and clippy -p roko-cli -p roko-learn -p roko-core -p roko-agent -p roko-gate -p roko-serve --no-deps -D warnings clean; lib tests pass: roko-cli 3133, roko-core 1938, roko-learn 1196, roko-serve 958, roko-gate 689, roko-agent 2257 (its one failure, a_timed_out_attempt_reports_the_usage_it_streamed, is a load flake at load 77 that passes alone). Verify: multi_objective_observations_advance_the_stage_and_the_frontier passes (roko-learn lib)."
+++

## Problem

The Graph path's feedback sinks update the cascade router through `observe_multi_objective_outcome` (`roko-learn/src/cascade_router.rs:1823-1863` on `work/bug-f68404`). That method touches neither `stage_tracking` nor `pareto_frontier`. The stage advances only in `check_stage_transition` (:450), and routing reads `current_stage()` (:408). On Graph-only learning, the router's stage lags until the router is reloaded, and the Pareto frontier isn't refreshed with the new observations.

## Why it matters

Cybernetic core (epic spec-6ac537): the stage decides how the router explores. If it lags, learning from Graph runs doesn't change routing when it should.

## Where

`observe_multi_objective_outcome`, `check_stage_transition` and the Pareto frontier refresh in `cascade_router.rs`.

## Plan

1. After recording an observation, run the same stage check and frontier refresh that the other observation paths run (or that the reload runs).
2. Add `multi_objective_observations_advance_the_stage_and_the_frontier`.

## Done when

- [ ] Observations made through the Graph sinks advance the stage and refresh the frontier without a reload.
- [ ] The `[[verify]]` command passes.

## Notes

- Implemented on `work/bug-f81e9b` at `9d39ea2d7`; cargo verification deferred to the batch check.
  `observe_multi_objective_outcome` now ends as `observe_internal` does, with `refresh_pareto_frontier_if_needed`
  and `check_stage_transition`. `record_override_outcome` already went through `observe_internal`.
