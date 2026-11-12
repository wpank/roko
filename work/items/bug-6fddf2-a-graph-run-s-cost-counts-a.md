+++
id = "bug-6fddf2"
kind = "bug"
title = "A Graph run's cost counts a try twice when the hub gets its cost row twice: roko serve's run summary and the dashboard snapshot add up `cost_usd` rows that name no try (rehearsal run b2372e07 read $0.0086, with \"$0.0032 helper calls\" on the Hermes run card, for $0.0054 of tries)"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-serve/runs", "roko-cli/graph_task_dispatch", "roko-core/dashboard_snapshot"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
source = "round-v8-run-analyst-v8#dispatch-v2-diagnosis"
discovered_from = "agent:run-analyst-v8 (round v8, the lead's ask, 2026-11-12)"
anchors = ["crates/roko-serve/src/routes/runs.rs::summarize_events", "crates/roko-serve/src/routes/runs.rs::Efficiency", "crates/roko-cli/src/graph_task_dispatch/feedback.rs::emit_feedback", "crates/roko-cli/src/runner/tui_bridge.rs::try_efficiency_event", "crates/roko-core/src/dashboard_snapshot.rs::DashboardSnapshot"]
links = { depends_on = [], blocks = [], related = ["bug-2839c4", "gap-1430ea", "bug-11e54b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_graph_runs_summary_counts_each_try_once' crates/roko-serve/src && cargo test -p roko-serve --lib a_graph_runs_summary_counts_each_try_once"
+++

## Problem

A Graph run publishes each try's cost on its hub once, after the try settles: an `efficiency_event` with metric
`cost_usd` and no `attempt` (`emit_feedback`). roko serve's run summary adds up every such row in the run's index,
and the dashboard snapshot adds up every one it hears. Neither can tell one try's row published twice from two tries,
so a doubled row is charged twice.

Round v8's rehearsal run b2372e07 (roko bd93521ee, 2026-11-12; two tasks, each passed on its first try) has one:

| Record | Task 2 | Task 1 | Run |
|---|---|---|---|
| `attempt_settled.cost_usd`, `learn/costs.jsonl` | $0.0022777 | $0.0031639 | $0.0054416 |
| The plan's checkpoint, `committed.spent_micro_usd` | | | 5442 |
| `cost_usd` rows on the hub | 1 | 2 | $0.0086054 |

- Task 1's `cost_usd` row is in the run's index twice (rows 285 and 286, both at `ts_millis` 1794482212073) and in
  the workspace's `.roko/events.jsonl` twice (lines 460 and 461). The four token rows that `token_usage` published
  just before it, in the same call, are there once each.
- roko serve's run summary (`GET /api/runs/{id}/summary`, and MCP `run_status`, which returns the same summary) gave
  `cost_usd` 0.0086054 and an empty `side_call_usd`.
- The Hermes plugin's run card showed $0.009. Its line comparing the run with claude-sonnet-4-6 at list prices added
  "plus $0.0032 helper calls": with no side calls, the card takes the run's cost less its tries', and that is task
  1's try counted again.

How often: 1 run of 26. The 24 round-v7 Undertow runs have no doubled row of any efficiency metric (729,188 index
rows); of v8's two rehearsal runs, this is the one.

## Why it matters

Goal `truth`: a cost number has one meaning. The round v8 guides say every cent is counted. A doubled publish is
rare, but nothing downstream catches it: the row names no try, the summary trusts the sum, and the plugin calls the
excess helper calls.

## Where

- `crates/roko-cli/src/graph_task_dispatch/feedback.rs::emit_feedback` publishes `token_usage`, then
  `efficiency_event(plan, task, "cost_usd", cost)`. The try is known there (`settled.attempt_key()`).
- `crates/roko-cli/src/runner/tui_bridge.rs::try_efficiency_event`: `efficiency_event` calls it with
  `attempt: None`. Side calls already pass their try (gap-1430ea).
- `crates/roko-serve/src/routes/runs.rs::summarize_events`: `Efficiency::add` sums the `cost_usd` rows, and the
  `graph_spend` branch makes the run's cost that sum plus its side calls. The branch's comment says a Graph run
  records its spend only as efficiency events, but each try's `attempt_settled` row carries its `cost_usd`, keyed by
  plan, task and attempt, and the summary already reads those rows.
- `crates/roko-core/src/dashboard_snapshot.rs::DashboardSnapshot`: in `apply`, a `cost_usd` row adds to the agent's,
  the plan's and `stats.cost_usd_total`, so the dashboard and the portal show the doubled figure too.

## Current state

- **The second publish's source isn't established.** On the Graph path only `emit_feedback` publishes a try's
  `cost_usd`; `plan_authoring.rs` publishes plan writing's. Its token rows weren't doubled. `StateHub::publish`
  appends to `events.jsonl` and broadcasts once per call, so the hub was handed the row twice.
- **What set task 1's try apart:** it was the plan's last task, and the only one with a lesson written
  (`knowledge_written`). Both tries ran in parallel in a run serve started, and every verify step was taken from the
  try's own checks. roko.log has nothing between task 1's verify (11:16:51.97Z) and its merge (11:16:52.70Z).
- **Already right:** `attempt_settled`, `call_cost`, `learn/costs.jsonl` and the plan checkpoint.

## Plan

1. Key the row: publish the try's cost with `try_efficiency_event(plan, task, Some(attempt), "cost_usd", cost)`.
2. In `summarize_events`, count each try's spend once: its `attempt_settled.cost_usd`, else its last `cost_usd` row
   (by the row's `attempt`, or the running try's, as `refused_in_try` finds it). Side calls stay as they are. A
   doubled row then changes nothing.
3. In `DashboardSnapshot::apply`, a `cost_usd` row that names its try replaces that try's earlier one instead of
   adding to it.
4. Find the second publish with a test that a settled try publishes one `cost_usd` row, on the path b2372e07 took:
   two tasks in parallel, checks reused, a lesson written for the last task.

## Done when

`a_graph_runs_summary_counts_each_try_once` passes: a run index with one try's `cost_usd` row twice and its
`attempt_settled` once gives the settled cost as the run's `cost_usd`, with no side calls.

## Notes

- **Not this:** the `dispatch_v2` rows in `learn/efficiency.jsonl` (bug-2839c4). No reader a person sees sums them;
  that item has the census of readers.
- **The plugin's half,** what its card counts, is the plugin's own work, and its author has been told.
- **Evidence:** round v8's run-analyst-v8 (`tmp/hermes-roko-v8/findings/run-analyst-v8.md`). All of it is $0, read
  from the rehearsal workspace's records.
