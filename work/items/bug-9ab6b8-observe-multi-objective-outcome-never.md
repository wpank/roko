+++
id = "bug-9ab6b8"
kind = "bug"
title = "observe_multi_objective_outcome never advances stage_tracking or refreshes the Pareto frontier"
status = "open"
triage = "unverified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/cascade_router"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-router2's report, branch work/bug-f68404 at 732728ee8)"
anchors = ["crates/roko-learn/src/cascade_router.rs"]
lane = "rust-hot"
parent = "spec-6ac537"
links = { depends_on = [], blocks = [], related = ["bug-f68404", "bug-605a8a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn multi_objective_observations_advance_the_stage_and_the_frontier' crates/roko-learn/src/ && cargo test -p roko-learn --lib multi_objective_observations_advance_the_stage_and_the_frontier"
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
