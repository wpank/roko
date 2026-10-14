+++
id = "gap-5fda00"
kind = "gap"
title = "API-model attempts and prompt runs publish no live tool steps or tool results"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "M"
subsystem = ["roko-agent/tool_loop", "roko-cli/run", "roko-serve/routes"]
created = 2026-10-14
updated = 2026-10-14
last_verified = 2026-10-14
source = "e2e-v4 dogfood, roko-stream worker, 2026-10-14 (roko main 86d629363)"
anchors = ["crates/roko-agent/src/tool_loop/mod.rs::send_turn_streaming", "crates/roko-agent/src/immune_boundary.rs::drive_streaming_inner", "crates/roko-cli/src/run.rs::run_prompt", "crates/roko-cli/src/serve_runtime.rs::run_prompt_plan_on_local_runtime", "crates/roko-serve/src/runtime.rs::PromptPlanOptions", "crates/roko-serve/src/routes/mcp.rs", "crates/roko-serve/src/routes/run.rs"]
links = { depends_on = [], blocks = [], related = ["bug-0b7c81", "bug-264c41"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn api_model_tool_calls_reach_the_live_output' crates/roko-agent/ && cargo test -p roko-agent api_model_tool_calls_reach_the_live_output"

[[verify]]
command = "grep -rqw 'fn prompt_runs_take_the_live_output_setting' crates/roko-serve/ && cargo test -p roko-serve prompt_runs_take_the_live_output_setting"
+++

## Problem

An attempt on an API model (an OpenAI-compatible provider such as glm-5.1, or the Anthropic API) publishes no live
`tool_start` record while it runs, and under `[serve] live_agent_output = "trusted"` no live `tool_result` either:
a host watching `GET /api/runs/{id}/events` or the hub SSE sees the attempt silent until it ends. A Claude CLI
attempt shows each tool step live. Seen in the e2e-v4 dogfood (glm-5.1 runs in the tally workspace, roko main
86d629363): the plugin UI's live view stayed empty for every API-model attempt.

Prompt runs differ from plan runs as well: `roko run`, `POST /api/run` and the MCP `run_prompt` tool always run with
`LiveAgentOutput::ToolSteps`, whatever `[serve] live_agent_output` says, so a trusted server shows a prompt run's
tool steps but never its text, reasoning or tool results, while it shows all of them for a plan run.

## Why it matters

Goal `visibility`: a host shows what an attempt is doing as it does it. Most cheap execution runs on API models, so
their attempts were the ones with no live output. The stall watchdog reads the same live events (a tool step opens a
call, its tool result closes it), so API-model attempts also gave it nothing to tell a thinking model from a running
tool.

## Where

- `crates/roko-agent/src/tool_loop/mod.rs`: `ToolLoop::run_inner` and `send_turn_streaming`, the loop every API
  provider (OpenAI-compatible and Anthropic API) runs, tees each provider stream event to the caller.
- `crates/roko-agent/src/immune_boundary.rs::drive_streaming_inner`: the provider boundary turns a stream's
  `ToolCallEnd` into a `LiveAgentEvent::ToolStep` (and, trusted, an unscreened `ToolCallEnd`) and forwards
  `ToolResult` when trusted. The Graph watchdog's tap publishes them as live `tool_start` and `tool_result` records.
- `crates/roko-cli/src/run.rs::run_prompt` (hard-codes `LiveAgentOutput::ToolSteps`),
  `crates/roko-cli/src/serve_runtime.rs::run_prompt_plan_on_local_runtime`, `crates/roko-serve/src/runtime.rs`
  (`PromptPlanOptions`), `crates/roko-serve/src/routes/run.rs` and `crates/roko-serve/src/routes/mcp.rs`.

## Current state

OpenAI-compatible streams send a call as `ToolCallStart` plus `ToolCallDelta`s and never a `ToolCallEnd`, so the
boundary sees no ended call. No tool loop emits a `ToolResult` event: it runs the tools itself. The Claude CLI path
gets both from the CLI's stream-json. Plan runs started by serve pass `state.effective_live_agent_output()` through
`PlanRunOptions`; prompt runs have no such option.

## Plan

1. In the tool loop, after a streamed turn's calls are parsed and pass the ingress check, emit a `ToolCallEnd`
   (parsed id, name and arguments) for each call its stream did not end, and after the calls run (and untrusted
   results are screened) emit one `ToolResult` per call. Ids then match the tool audit's.
2. Give `PromptPlanOptions` and `PromptRun` a live-output setting; serve's `/api/run` and MCP `run_prompt` pass the
   server's effective setting, `roko run` the configured one (as `roko plan run` does).

## Done when

Both `[[verify]]` commands pass: a streamed OpenAI-compatible tool call reaches the live output as a tool step and,
trusted, its result; serve's prompt runs carry the server's live-output setting. A zero-cost serve smoke shows live
`tool_start` records with the right attempt for a prompt run and a plan run on a fake API model.

## Notes

The chat path (`dispatch_v2::run_agent_streaming`) reads the same stream: it gets one more tool-call event per API
call (with the real id) and, new, its tool output.
