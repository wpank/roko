+++
id = "gap-4e852d"
kind = "gap"
title = "No hub event says how a Graph attempt ended: publish attempt_settled per try (verdict, blame, rung standing, usage, cost, time)"
status = "done"
triage = "verified"
severity = "p1"
goal = "visibility"
size = "S"
subsystem = ["roko-core/dashboard"]
created = 2026-10-16
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "69361cb50"
source = "spec-v3 §10 R1 (tmp/hermes-roko-v6/design/spec-v3.md); round v6 brief"
anchors = ["crates/roko-cli/src/graph_task_dispatch/feedback.rs::publish_settlement", "crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent"]
lane = "rust-hot"
links = { depends_on = ["gap-3e770b"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn each_try_publishes_attempt_settled' crates/roko-cli/src && cargo test -p roko-cli --lib each_try_publishes_attempt_settled"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T11:16:38Z"
commit = "2d1276411"
forced = false
evidence = "2d1276411: publish_settlement publishes attempt_settled once each try counts toward the ladder standing: plan_id, task_id, attempt, rung, escalations, failures_on_rung, climbed, reason, verdict, blame, failed_check, detail, model, provider, usage {in, out, cache_read, reasoning}, cost_usd (what the plan budget counted), duration_ms and exhausted, in a boxed AttemptSettlement flattened into the event; roko show --follow prints a line per try, the snapshot logs it, and the workspace event log keeps a redacted excerpt of detail. each_try_publishes_attempt_settled passes. Live (pocket-synth, binary 69361cb50): one attempt_settled per try, e.g. SVF try 2: turn_cap, agent, escalations 1, failures_on_rung 0, climbed true, $0.066. Checks at 69361cb50 (the stack rebuilt on main 9df2f5948): fmt --check, clippy -D warnings (roko-core, roko-cli, roko-serve), lib tests roko-core 2038 passed, roko-serve 1052 passed, roko-cli 3769 passed with one known load flake (plan_check_publishes_start_heartbeat_and_finish, bug-8d8f98, passes alone); the [[verify]] passes."
+++

## Problem

No hub event says how a Graph attempt ended. Its verdict, ladder standing, usage and cost go to the run's
`attempts.jsonl`, but nothing on the hub (SSE, WebSocket, `.roko/events.jsonl`) carries them, so a host can only
guess a try's outcome from gate results and agent events. Spec-v3 §10 R1 asks for `attempt_settled`:
`{plan_id, task_id, attempt, rung, escalations, failures_on_rung, reason, blame, verdict, usage {in, out,
cache_read, reasoning}, cost_usd, duration_ms}`.

## Why it matters

The plugin's per-try climb (P2 `tasks[].climb`, `tasks[].lives`) and receipt (P3, by rung) need each try's outcome,
blame, rung standing and cost as it happens. Without it the Ladder cannot draw a ✕ for a failed try or count lives.

## Where

- `crates/roko-cli/src/graph_task_dispatch/feedback.rs::publish_settlement`: every settled attempt passes through
  it (batch and streaming paths, reflex attempts, harness failures), and it counts the attempt toward the task's
  ladder standing (`note_ladder_outcome`).
- `crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent`.

## Current state

`publish_settlement` sends `FeedbackEvent::AttemptSettled` to the feedback facade only; the hub sees nothing.

## Plan

1. Add `DashboardEvent::AttemptSettled` with R1's fields, plus `model`, `provider` and `failed_check` (the verify
   rung that failed it), and `exhausted`.
2. Publish it from `publish_settlement`, after `note_ladder_outcome`, so `escalations` and `failures_on_rung` are the
   task's standing once this try counts (a climb shows as `failures_on_rung = 0` and one more escalation).
   `reason` uses `route_decided`'s words (start, retry, climb, failover, pinned).
3. List it in every exhaustive match (`output_sink.rs`, roko-serve's bridge and health routes, the snapshot).

## Done when

`each_try_publishes_attempt_settled` shows one event per try with the verdict, blame, rung and standing of a task
that fails twice on the cheap rung and climbs; the `[[verify]]` command passes.

## Notes

The usage counts are `null` when the provider did not report them, never 0.
