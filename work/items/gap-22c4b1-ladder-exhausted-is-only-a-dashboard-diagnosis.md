+++
id = "gap-22c4b1"
kind = "gap"
title = "ladder_exhausted is only a dashboard diagnosis: publish it as a hub event"
status = "open"
triage = "unverified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["roko-core/dashboard"]
created = 2026-10-16
updated = 2026-10-16
source = "spec-v3 §10 R4 (tmp/hermes-roko-v6/design/spec-v3.md); round v6 brief"
anchors = ["crates/roko-cli/src/graph_task_dispatch/ladder.rs::report_ladder_exhausted", "crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn ladder_exhausted_is_a_hub_event' crates/roko-cli/src && cargo test -p roko-cli --lib ladder_exhausted_is_a_hub_event"
+++

## Problem

When a task fails on its top rung with no retries left, roko logs `ladder_exhausted` and publishes a dashboard
diagnosis (`ladder_exhausted:<plan>/<task>`), but no event of its own. A host that follows the event stream must
parse diagnosis ids to notice. Spec-v3 §10 R4 asks for a hub event, for the plugin's `⌈SPLIT OR REPLAN⌋` stamp.

## Why it matters

Exhausting the ladder is the one ladder outcome that needs a person (split or replan the task); it should be as
visible as a climb.

## Where

- `crates/roko-cli/src/graph_task_dispatch/ladder.rs::report_ladder_exhausted`.
- `crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent`.

## Current state

`report_ladder_exhausted` writes a tracing warning and `tui.diagnosis(…)`.

## Plan

Add `DashboardEvent::LadderExhausted {plan_id, task_id, attempt, rung, model, escalations, suggestion}` and publish
it beside the diagnosis (which stays, for the dashboard's diagnosis panel). List it in every exhaustive match.

## Done when

`ladder_exhausted_is_a_hub_event` shows the event once for a task that fails on its top rung with no retries left,
and never for a pinned task; the `[[verify]]` command passes.
