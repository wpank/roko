+++
id = "gap-22c4b1"
kind = "gap"
title = "ladder_exhausted is only a dashboard diagnosis: publish it as a hub event"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["roko-core/dashboard"]
created = 2026-10-16
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "69361cb50"
source = "spec-v3 §10 R4 (tmp/hermes-roko-v6/design/spec-v3.md); round v6 brief"
anchors = ["crates/roko-cli/src/graph_task_dispatch/ladder.rs::report_ladder_exhausted", "crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn ladder_exhausted_is_a_hub_event' crates/roko-cli/src && cargo test -p roko-cli --lib ladder_exhausted_is_a_hub_event"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T11:16:39Z"
commit = "69361cb50"
forced = false
evidence = "69361cb50: note_ladder_outcome says whether the try exhausted the ladder, and publish_settlement then publishes ladder_exhausted {plan_id, task_id, attempt, rung, model, escalations, suggestion} right after that try's attempt_settled; the ladder_exhausted diagnosis stays. ladder_exhausted_is_a_hub_event passes (once for a task that fails on its top rung with no retries left, never for a pinned task). Checks at 69361cb50 (the stack rebuilt on main 9df2f5948): fmt --check, clippy -D warnings (roko-core, roko-cli, roko-serve), lib tests roko-core 2038 passed, roko-serve 1052 passed, roko-cli 3769 passed with one known load flake (plan_check_publishes_start_heartbeat_and_finish, bug-8d8f98, passes alone); the [[verify]] passes."
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
