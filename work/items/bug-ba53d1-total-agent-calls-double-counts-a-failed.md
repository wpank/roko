+++
id = "bug-ba53d1"
kind = "bug"
title = "total_agent_calls double-counts a failed-over attempt (failover's dashboard upsert reuses AgentSpawned)"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph-execution", "roko-cli/graph-task-dispatch"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-6 follow-up reports 2026-10-03 (PK03 gap-d254a3)"
discovered_from = "gap-d254a3"
anchors = ["crates/roko-cli/src/graph_execution/event_log.rs", "crates/roko-cli/src/graph_task_dispatch/failover.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn failed_over_attempt_counts_one_agent_call' crates/roko-cli/ && cargo test -p roko-cli failed_over_attempt_counts_one_agent_call"
+++

## Problem

`total_agent_calls` counts every `DashboardEvent::AgentSpawned` the Graph run's event log observes
(`crates/roko-cli/src/graph_execution/event_log.rs:485`: `DashboardEvent::AgentSpawned { .. } => self.agent_calls += 1`,
surfaced as `total_agent_calls: self.agent_calls` at line 568). But failover republishes a *second*
`AgentSpawned` event for the same logical attempt when it substitutes a model after a refusal
(`crates/roko-cli/src/graph_task_dispatch/failover.rs:265-280`): once `refusals` is non-empty, it calls
`tui.agent_spawned(row.agent_id, ...)` with the **same `agent_id`** the attempt's original dispatch opened,
explicitly to *rename* the dashboard row to the model that actually ran — the surrounding comment says so
verbatim: "the dashboard row the attempt opened with the planned model names the one that runs instead; the
hub upserts the row by its agent id (backlog 1128)." It is an upsert of one row, not a second agent call.

`event_log.rs`'s counter has no way to tell an upsert-by-existing-`agent_id` apart from a genuinely new spawn:
it just matches on the event variant and increments. So one attempt that fails over once is counted as two
agent calls.

## Why it matters

Goal `truth`: `total_agent_calls` is a reported run metric (`roko status`, `roko diagnose`, the dashboard
summary, `roko-learn::run_metrics`) that operators and any cost-per-call analysis read as "how many provider
dispatches this run made." A run with frequent failover (exactly the runs most worth watching) over-reports
its own agent-call count, which skews any $/call or calls-per-task figure derived from it.

## Where

- Counter: `crates/roko-cli/src/graph_execution/event_log.rs::EventLog` (or its containing struct) — the
  `agent_calls` field (line 464) and its increment on `DashboardEvent::AgentSpawned` (line 485).
- Republish: `crates/roko-cli/src/graph_task_dispatch/failover.rs`, inside the model-substitution loop, the
  `tui.agent_spawned(row.agent_id, &spec.plan_id, task_id, 0, row.role, &target.model_slug, &target.provider_id)`
  call (around lines 268-280), gated on `!refusals.is_empty()`.

## Current state

Unfixed. `event_log.rs` has no set/map of `agent_id`s already seen to dedupe against; every `AgentSpawned`
event increments the counter regardless of whether its `agent_id` already opened a row.

## Plan

1. Track the set of `agent_id`s that have already incremented `agent_calls` (a `HashSet<String>` alongside
   the counter, or equivalent) in the event-log aggregator.
2. On `DashboardEvent::AgentSpawned { agent_id, .. }`, increment `agent_calls` only the first time that
   `agent_id` is seen; a repeat (the failover upsert) updates whatever per-row state it's meant to (model,
   provider) without incrementing the count.
3. Add a regression test: one attempt whose dispatch fails over once (two `AgentSpawned` events, same
   `agent_id`) ends the run with `total_agent_calls == 1`.

## Done when

- A failed-over attempt (one logical dispatch, two `AgentSpawned` events on the same `agent_id`) counts once
  toward `total_agent_calls`.
- The `[[verify]]` command passes.

## Notes

- Do not change the *upsert* behavior itself (renaming the row to the model that actually ran is correct and
  wanted, per backlog 1128) — only the counting.
- Distinct from `bug-c55f1c` (an earlier batch's finding: a provider exhaustion refusal recorded twice between
  the bridge and failover's `record_exhaustion`) — that is a cost/health-ledger double-count; this is a
  dashboard/metrics double-count. Different code paths, same general failover-retrofit-after-the-fact pattern.
