+++
id = "q-61b3f8"
kind = "question"
title = "Should plan-set parallelism stay opt-in at 1, or run independent plans side by side by default?"
status = "open"
triage = "unverified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-7c9e48"
anchors = ["crates/roko-cli/src/graph_execution/plan_set.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-7c9e48"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -n 'max_parallel_plans' crates/roko-core/src/config/"
+++

## Problem

gap-7c9e48 found that the side-by-side plan scheduler works and is tested (`independent_plans_run_side_by_side`, plus the new diamond test), but the default still runs one plan at a time. Turning it on by default changes cost and disk use per run.

## Plan

Will decides the default. If it changes, update the config default and docs, and keep the order test for `max_parallel_plans = 1`.

## Done when

- The decision is recorded, and the default matches it.

## Notes

- Reported on 2026-10-01 by wk-planrun, working on gap-7c9e48, during the evening close-out round.
