+++
id = "gap-c31dd6"
kind = "gap"
title = "affect_updated and cascade_router_updated are defined hub events that nothing publishes"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-core/dashboard_snapshot"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "round-v6-learning-research#4b.10"
discovered_from = "agent:learning-research (round v6, 2026-10-16)"
anchors = ["crates/roko-cli/src/runner/tui_bridge.rs::affect_updated", "crates/roko-cli/src/runner/tui_bridge.rs::cascade_router_updated", "crates/roko-core/src/dashboard_snapshot.rs::AffectUpdated", "crates/roko-cli/src/graph_execution/learning_commit.rs::commit_run_learning"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn daimon_and_router_updates_reach_the_hub' crates/roko-cli/ && cargo test -p roko-cli daimon_and_router_updates_reach_the_hub"
+++

## Problem

Two StateHub events are defined and documented in `DashboardEvent`, but nothing in a run publishes them:
- `affect_updated`: `TuiBridge::affect_updated` has no caller outside tests, although a run with `[daimon] enabled =
  true` updates affect after each labelled attempt, and the PlanCompleted daimon sink saves it (gap-a960a1);
- `cascade_router_updated`: `TuiBridge::cascade_router_updated` has no caller. The dashboard reads the router file from
  disk instead.

Found in round v6 (`tmp/hermes-roko-v6/findings/learning-research.md` §1.5, §4b.10) while listing what a live UI can
show of roko's learning. A third one, `efficiency_trend_updated`, is tracked with the cost-trend work, not here.

## Why it matters

Goal `visibility`. A UI following the hub (the TUI, roko serve's SSE, the Hermes plugin's live relay) cannot show
affect or router state changing during a run; it has to poll files. Dead variants also mislead anyone writing a
consumer.

## Where

- `crates/roko-cli/src/runner/tui_bridge.rs::affect_updated` and `crates/roko-cli/src/runner/tui_bridge.rs::cascade_router_updated`:
  the helpers with no caller.
- `crates/roko-core/src/dashboard_snapshot.rs::AffectUpdated`, `CascadeRouterUpdated`: the variants.
- Where state changes:
  - the daimon update in Graph feedback (`crates/roko-cli/src/graph_task_dispatch/feedback.rs`, the daimon step);
  - the router merge at run end (`crates/roko-cli/src/graph_execution/learning_commit.rs::commit_run_learning`).

## Current state

Both helpers and variants exist. Only the calls are missing.

## Plan

1. Publish `affect_updated` after each daimon update (only when `[daimon] enabled`), and `cascade_router_updated` after
   the run-end router commit.
2. Test `daimon_and_router_updates_reach_the_hub`: a fake-provider run with the daimon on publishes one
   `affect_updated` per labelled attempt and one `cascade_router_updated` at its end.
3. If either is judged not worth publishing, delete it from `DashboardEvent` and its consumers instead, and say so in
   the closure.

## Done when

The `[[verify]]` command passes, or the closure records the deletion and why.

## Notes

`cascade_router_updated` carries a whole snapshot as JSON. Keep the event small: publish the digest and counts, and let
consumers read the file for the rest.
