+++
id = "gap-2a3298"
kind = "gap"
title = "Graph route decisions are written only to decisions.jsonl, so no event stream shows which model an attempt got and why"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-14
updated = 2026-10-14
last_verified = 2026-10-14
last_verified_rev = "ca3dfad86"
source = "roko-loops worker for the hermes-plugin-roko live UI, 2026-10-14 (roko main 216f88c84)"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs", "crates/roko-cli/src/graph_task_dispatch/decision_log.rs::record_planned_attempt", "crates/roko-cli/src/graph_task_dispatch/failover.rs", "crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn route_decisions_are_published' crates/roko-cli/ && cargo test -p roko-cli route_decisions_are_published"

[closed]
at = 2026-10-14
at_ts = "2026-10-14T08:58:00Z"
commit = "ca3dfad86"
by = "commit trailer"
executor = "claude-agent"
size = "M"
claimed_at = "2026-10-14T08:26:38Z"
forced = false
evidence = "ca3dfad86: the planned Graph attempt publishes route_decided (plan_id, task_id, attempt, model, provider, stage, reason, source, best 5 candidates) on the run's hub before its agent_spawned (graph_task_dispatch/decision_log.rs route_event), and failover publishes one with source failover (failover.rs failover_route_event); DashboardEvent::RouteDecided and RouteCandidate in roko-core, recorded by WorkspaceEventLog, documented in docs/v3/26-HTTP-API.md. Verify: cargo test -p roko-cli route_decisions_are_published passes."
+++

## Problem

When a Graph attempt is routed, the decision (the model picked, by which stage and source, why, and the candidates
with their scores and probabilities) is written only to `.roko/runs/<run>/decisions.jsonl`. No event carries it:
the run's event index, the per-run SSE (`/api/runs/{id}/events/stream`) and the hub stream (`/api/events`) show
`agent_spawned` with a model name and nothing about why. A failover that switches the model mid-attempt re-sends
`agent_spawned` with the new model, again without a reason.

## Why it matters

Routing is roko's learning loop L3 acting on loop L1: the router learns from verified results and chooses the next
attempt's model. A host following a run (the Hermes plugin's run view, `roko dashboard`) cannot show that loop at
work without reading files from the workspace (goal `visibility`).

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs`: dispatch plans the attempt (`plan_dispatch`, `record_planned_attempt`)
  and publishes `agent_spawned` before the provider call.
- `crates/roko-cli/src/graph_task_dispatch/decision_log.rs::record_planned_attempt`: stamps the route row.
- `crates/roko-cli/src/graph_task_dispatch/failover.rs`: re-publishes `agent_spawned` when failover runs another model.
- `crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent`; `roko_learn::routing_log::RoutingDecisionLog`.

## Current state

`TuiBridge::model_selected` (an `event_log_entry`) exists but has no Graph caller.

## Plan

1. Add `DashboardEvent::RouteDecided {plan_id, task_id, attempt, model, provider, stage, reason, source,
   candidates}` (top 5 candidates by score: model, provider, score, p, eligible, ineligible_reason).
2. Publish it from the route row of each planned attempt, before `agent_spawned`.
3. On failover, publish it again with `source: "failover"` and the reason the planned model was passed over.
4. Keep it in every exhaustive match (snapshot, output sink, serve bridge, health type names) and document it.

## Done when

`[[verify]]` passes: a planned attempt's route decision is published with its candidates, in the order dispatch
plans it.
