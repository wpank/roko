+++
id = "gap-4e852d"
kind = "gap"
title = "No hub event says how a Graph attempt ended: publish attempt_settled per try (verdict, blame, rung standing, usage, cost, time)"
status = "open"
triage = "unverified"
severity = "p1"
goal = "visibility"
size = "S"
subsystem = ["roko-core/dashboard"]
created = 2026-10-16
updated = 2026-10-16
source = "spec-v3 §10 R1 (tmp/hermes-roko-v6/design/spec-v3.md); round v6 brief"
anchors = ["crates/roko-cli/src/graph_task_dispatch/feedback.rs::publish_settlement", "crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent"]
lane = "rust-hot"
links = { depends_on = ["gap-3e770b"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn each_try_publishes_attempt_settled' crates/roko-cli/src && cargo test -p roko-cli --lib each_try_publishes_attempt_settled"
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
