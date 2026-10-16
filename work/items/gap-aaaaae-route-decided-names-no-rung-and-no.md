+++
id = "gap-aaaaae"
kind = "gap"
title = "route_decided names no rung and no ladder reason (start, retry, climb, failover, pinned), and never carries the self-model's forecast"
status = "open"
triage = "unverified"
severity = "p1"
goal = "visibility"
size = "M"
subsystem = ["roko-core/dashboard"]
created = 2026-10-16
updated = 2026-10-16
source = "round v6 brief; tmp/hermes-roko-v6/findings/learning-research.md items 5 and 6; spec-v3 §4.3, §10 P2"
anchors = ["crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent", "crates/roko-cli/src/graph_task_dispatch/decision_log.rs::route_event", "crates/roko-cli/src/graph_task_dispatch/failover.rs::failover_route_event", "crates/roko-cli/src/graph_task_dispatch/attempt.rs::AttemptContext"]
lane = "rust-hot"
links = { depends_on = ["gap-3e770b"], blocks = [], related = ["gap-2a3298"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn route_decided_names_its_rung_step_and_reason' crates/roko-cli/src && grep -rqw 'fn route_decided_carries_the_self_model_forecast' crates/roko-cli/src && cargo test -p roko-cli --lib route_decided_"
+++

## Problem

A host following a Graph run cannot tell where an attempt sits on the model ladder from its `route_decided` event.
The event carries the model, provider, stage, the route's `reason` text, `source` and candidates, but no rung, no
step and no word for why this try runs there (start, retry, climb, failover or pinned). The Hermes plugin reads a
`rung` field that never comes (round v6 brief; `tmp/hermes-roko-v6/findings/learning-research.md` item 5), and its
Ladder view (spec-v3 §4.3, P2 `tasks[].climb`) has to guess a climb from model names.

The self-model's forecast (M3, written to `predictions.jsonl` before the route decision) never reaches the hub
either (learning-research item 6), so the plugin cannot show what the self-model would have chosen.

## Why it matters

The pitch's hero is the visible climb (spec-v3 §4.3, §6: retry, climb and failover each have their own motion).
Without the rung and the reason on the event, a host cannot draw it from the stream.

## Where

- `crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent`: `RouteDecided`.
- `crates/roko-cli/src/graph_task_dispatch/decision_log.rs::route_event`, published by `record_planned_attempt`
  right after `record_attempt_ladder` (`ladder.rs`), which knows the attempt's rung and step.
- `crates/roko-cli/src/graph_task_dispatch/failover.rs::failover_route_event`: the event failover publishes when it
  passes the planned model over.
- `crates/roko-cli/src/graph_task_dispatch/attempt.rs::AttemptContext::record_prediction`: the forecast, recorded
  before routing.

## Current state

`AttemptLadder` (the verdict's `ladder`) records the rung, the step and a `LadderReason` (start, hint, escalated,
pinned, failover, self_model), but a second try on the same rung after a climb still says `escalated`, so it cannot
tell a retry from a climb. The route event is built from the route row alone.

## Plan

1. `RouteDecided` gains `rung` (the rung name, `null` when the ladder did not route the attempt), `ladder_step`
   (rungs climbed above the start rung) and `ladder_reason`: `start`, `retry`, `climb`, `failover` or `pinned`. All
   default when absent, so old readers and old event logs still parse.
2. Work out the reason when the attempt is routed: its first try is `start`; a try on a higher step than the task's
   last try is `climb`, any other later try `retry`; a pinned model is `pinned`; the event failover publishes is
   `failover`, with the rung the substitute runs on. Keep it on the attempt, so `attempt_settled` can say the same.
3. `RouteDecided` gains `forecast` when the self-model forecast the attempt: its mode, its action, the model it would
   choose, and per candidate the model, provider, `p_vs` and `cost_q50` (not `cost_q90`, which is wild for arms with
   no history), highest `p_vs` first.

## Done when

`route_decided_names_its_rung_step_and_reason` shows `start`, `retry`, `climb` with the right rung and step on a
task that climbs, and `pinned` on a pinned task; the forecast test shows a forecast on the event; the `[[verify]]`
command passes.

## Notes

- Do not edit `graph_task_dispatch/self_model.rs` (claimed by gap-b38ed2): take the forecast where
  `AttemptContext::record_prediction` receives it.
- plugin-runs reads these shapes: list them in the round's findings.
