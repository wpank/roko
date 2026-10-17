+++
id = "bug-c7de28"
kind = "bug"
title = "Prompt runs started through roko serve name their agent and model 'claude' on the hub (agent_spawned, every heartbeat, the event log), whatever model runs"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-serve/routes"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "round-v6-dogfood-will#R3-7"
discovered_from = "agent:dogfood-will (round v6 run 3, 2026-10-16)"
anchors = ["crates/roko-serve/src/routes/run.rs::prepare_run", "crates/roko-serve/src/routes/run.rs::run_with_heartbeats"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'unwrap_or(\"claude\")' crates/roko-serve/src/routes/run.rs && grep -rqw 'fn prompt_run_events_name_no_default_vendor' crates/roko-serve/ && cargo test -p roko-serve prompt_run_events_name_no_default_vendor"
+++

## Problem

A prompt run started through roko serve tells the hub its agent is "claude", whatever model runs. The routes are
`POST /api/run` and the MCP `run` tool, which Hermes' `roko_run` calls. Before its Graph plan routes anything, the run
publishes:
- `agent_spawned {agent_id: "claude", model: "claude", attempt: 0, role: "run", plan_id: "run-<8>"}`;
- event-log entries `▶ claude: <prompt>`, and a matching entry with the run's mark at the end;
- `agent_heartbeat {agent_id: "claude", plan_id: "run-<8>"}`, every few seconds for the whole run.

Round v6 dogfood run 3 (2026-10-16, roko 47b3cdd20) showed this:
- Both chat runs, `913dc67f` and `d955cf89`, ran on gpt-oss-120b and gpt-5.4-mini, as their `route_decided` events
  show.
- The claude CLI is not even installed in that workspace (it is fenced).
- The hub still named "claude" as the agent. See `tmp/hermes-roko-v6/dogfood-will/logs/hub-1.jsonl` and
  `tmp/hermes-roko-v6/dogfood-will/evidence/run3/events-by-run/913dc67f-f835-45d0-a004-6163a132b70a.jsonl.gz`.

## Why it matters

- Goal `visibility`, and the pitch: roko is vendor-neutral, yet a dashboard or plugin that shows the spawned agent's
  model shows "claude" for every prompt run.
- The synthetic `run-<8>` plan id also duplicates the Graph plan id (`913dc67f-…`) that the same run's real events use.

## Where

- `crates/roko-serve/src/routes/run.rs::prepare_run`:
  `let agent_label = agent_target.as_deref().unwrap_or("claude");`. The label feeds `AgentSpawned`, both
  `EventLogEntry` messages and `run_with_heartbeats`.
- `crates/roko-serve/src/routes/run.rs::run_with_heartbeats`.

## Current state

The label is a leftover default from when a prompt run was one Claude CLI call. Today the prompt becomes a one-task
Graph plan, which publishes its own `route_decided` and `agent_spawned` for each attempt, with the real model.

## Plan

1. Use a neutral label for the run-level agent, such as `roko` or `run`. Leave `model` empty, or set it to the agent
   target when one was given. Never use a vendor name.
2. Or drop the run-level `AgentSpawned` and heartbeats, since the Graph plan publishes them per attempt. Check first
   that neither the TUI nor the Hermes plugin depends on them for prompt runs.
3. Add the test `prompt_run_events_name_no_default_vendor`: a prompt run with no agent target publishes no event whose
   agent or model is "claude".

## Done when

The `[[verify]]` passes. Its static part checks that `run.rs` no longer contains `unwrap_or("claude")`.

## Notes

A test in `crates/roko-serve/src/routes/run.rs` calls `run_with_heartbeats(&hub, "claude", …)` (around line 1243).
Update it to the new label.
