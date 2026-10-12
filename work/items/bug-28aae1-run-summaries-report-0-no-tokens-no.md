+++
id = "bug-28aae1"
kind = "bug"
title = "Run summaries report $0, no tokens, no gates and no start or end time for Graph runs, and list a phantom empty plan and task"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "M"
subsystem = ["roko-serve/runs"]
created = 2026-10-12
updated = 2026-10-12
last_verified = 2026-10-12
source = "hermes-plugin-roko 0.2.0 UI work, roko serve probe 2026-10-11 (scratchpad roko-ui-data.md)"
anchors = ["crates/roko-serve/src/routes/runs.rs::summarize_events", "crates/roko-cli/src/graph_execution/event_log.rs::WorkspaceLogWriter"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn summary_counts_cost_tokens_gates_and_times_of_graph_runs' crates/roko-serve/ && cargo test -p roko-serve summary_counts_cost_tokens_gates_and_times_of_graph_runs"

[[verify]]
command = "grep -rqw 'fn index_rows_carry_the_time_they_were_published' crates/roko-cli/ && cargo test -p roko-cli index_rows_carry_the_time_they_were_published"
+++

## Problem

For every Graph run (plan runs, and prompt runs, which are one-task plans), `GET /api/runs/{id}/summary`,
`GET /api/runs/{id}`, `/metrics` and `/api/dashboard/runs` report `cost_usd: 0.0`, zero tokens and zero gates,
and `started_at`/`finished_at` are `null`. Their `plans` and `tasks` lists also hold a phantom `""` entry.

Observed on 2026-10-11 (roko serve `856ed7570`), run `90cc6113`: the summary said `cost_usd 0.0`, `gates 0/0`,
`started_at null`, while its event index held `efficiency_event` rows summing to $0.01027 (3,352 input,
655 output, 10,368 cache-read tokens) and a passing `gate_result` for `verify[0:test]`.

## Why it matters

Every client of the run API (the portal, chat hosts over `/mcp` `run_status`, roko's own dashboard) shows a
paid, gated run as free and ungated. hermes-plugin-roko re-sums the events itself to work around it. Goal:
live visibility.

## Where

- `crates/roko-serve/src/routes/runs.rs::summarize_events` reads cost only from `inference_completed`,
  `agent.turn_completed`, `agent_completed` or `run_completed`, tokens only from `inference_completed` or
  `agent.token_usage`, and counts a gate only when its event type contains `completed`, `passed` or `failed`.
  Graph runs record cost and tokens as `efficiency_event` rows and gates as `gate_result`.
- It takes `started_at` from `run.started`/`workflow_started`/`run_started` and a timestamp on the end event;
  Graph runs emit `plan_set_loaded`/`plan_started`, and their durable index rows carry no time at all.
- `crates/roko-cli/src/graph_execution/event_log.rs::WorkspaceLogWriter::record` writes each hub event to the
  run's index without the envelope's `ts_millis`.
- `event_log_entry` rows carry `plan_id: ""` and `task_id: ""`.

## Current state

The live hub stream (`/api/events`) adds `ts_millis` to each event; the durable index does not. The summary's
`first_gate_failure` milestone already reads `gate_result.passed`.

## Plan

1. `WorkspaceLogWriter::record`: add `ts_millis` from the envelope to each index row, as `/api/events` does.
2. `summarize_events`: sum `efficiency_event` metrics (`cost_usd`, `input_tokens`, `output_tokens`,
   `cache_read_tokens`, `cache_write_tokens`) when no other cost source reported; count `gate_result` rows as gate
   results; take `started_at` from the first timestamped row and `finished_at` from `run_completed`'s; read
   `ts_millis` wherever a millisecond time is read; skip empty plan and task ids.
3. A test feeding a Graph run's real event shapes.

## Done when

- A Graph run's summary reports its real cost, tokens, gate counts and start and end times, and lists no `""`
  plan or task.
- The `[[verify]]` test passes.
