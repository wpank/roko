+++
id = "gap-4f6cbe"
kind = "gap"
title = "Graph runs never publish their delivery events, so roko serve and hosts cannot say how a delivery ended"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-cli/delivery"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "tmp/hermes-roko-v5/findings/plugin-runs.md (plugin-runs, 2026-10-16)"
discovered_from = "bug-7b62ff follow-up 2359ba4ec: a cancelled delivery is recorded only in the checkpoint"
anchors = ["crates/roko-cli/src/graph_execution/delivery.rs::drive_delivery", "crates/roko-serve/src/routes/runs.rs::delivery_end"]
links = { depends_on = [], blocks = [], related = ["bug-7b62ff"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_graph_run_publishes_its_delivery_end' crates/roko-cli && cargo test -p roko-cli a_graph_run_publishes_its_delivery_end"
+++

## Problem

A Graph run never publishes its delivery events. roko-graph defines `GraphExecutionEvent::DeliveryStarted`,
`DeliveryCompleted` and `DeliveryFailed` (`crates/roko-graph/src/events.rs`), but they are built only in tests
(the `tests` module from line 875); nothing on the plan path publishes them. `runtime_event_adapter.rs` maps them
to `graph.delivery` extension events, and `roko serve` (`routes/runs.rs::delivery_end`) turns a `graph.delivery`
`completed` or `failed` event into the run's `delivery` milestone, so that milestone never appears.

The delivery state machine (`graph_execution/delivery.rs::drive_delivery`) advances Prepared → Queued → Merged →
Delivered, Conflict, RegressionFailed or, since 2359ba4ec, Cancelled, and records the state only in the plan
checkpoint's delivery receipt and the batch record. A host therefore cannot tell a delivered run from one whose
merge a cancel or a failed regression check took back. The Hermes plugin infers it from the branch and the batch
(plugin-runs, 2026-10-16).

## Why it matters

A run whose merge the batch took back reads as partial work instead of "delivery cancelled" or "the regression
check failed", and `roko serve`'s run milestones skip the delivery step.

## Where

- `crates/roko-cli/src/graph_execution/delivery.rs::drive_delivery`: where each state change happens.
- `crates/roko-cli/src/graph_execution/runtime_event_adapter.rs`: maps the events to `graph.delivery`.
- `crates/roko-serve/src/routes/runs.rs::delivery_end`: reads `completed` and `failed` only.

## Plan

1. Publish `DeliveryStarted` when a delivery is prepared, and `DeliveryCompleted` or `DeliveryFailed` when it
   reaches a terminal state, carrying that state (delivered, conflict, regression_failed, cancelled) and what
   stopped it.
2. Teach `delivery_end` the terminal states, with cancelled read as a stop, not a failure.
3. Test end to end: a Graph run publishes its delivery's start and end, a delivery a cancel stopped publishes
   cancelled, and `roko serve`'s run milestones say "delivery cancelled".

## Done when

`a_graph_run_publishes_its_delivery_end` passes (the `[[verify]]` command).

## Notes

Found by plugin-runs while making a cancelled delivery read as a stop in the Hermes plugin
(`tmp/hermes-roko-v5/findings/plugin-runs.md`, last section).
