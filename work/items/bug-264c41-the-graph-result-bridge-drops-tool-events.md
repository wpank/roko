+++
id = "bug-264c41"
kind = "bug"
title = "The Graph result bridge drops tool events, so Claude CLI attempts record no tool calls and tool_result errors are lost"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "f4323cf9d"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-4d5e2d"
anchors = ["crates/roko-cli/src/graph_task_dispatch/feedback.rs", "crates/roko-agent/src/claude_cli_agent.rs", "crates/roko-cli/src/graph_task_dispatch/live_tool_calls.rs", "crates/roko-agent/src/tool_loop/mod.rs::StreamEventKind"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-4d5e2d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib cli_attempt_records_its_tool_calls"
+++

## Problem

`dispatch_events_from_result` projects only text, usage and lifecycle events into `dispatch.events`, so efficiency rows of Claude CLI attempts list no tool calls (gap-4d5e2d covered API attempts through the tool audit). `claude_cli_agent.rs` (around lines 1112 and 1148) also drops a tool_result's `is_error`.

## Plan

Record CLI tool calls from the live-output tap, which already sees ToolCallEnd and ToolResult, and carry `is_error` through. Add a test named `cli_attempt_records_its_tool_calls`.

## Done when

- `cargo test -p roko-cli --lib cli_attempt_records_its_tool_calls` passes.

## Notes

- Reported on 2026-10-01 by wk-settle, working on gap-4d5e2d, during the evening close-out round.
- 2026-10-01 (wk-settle): implemented on work/bug-f9ae3e; cargo verification deferred to the batch check.
  - `StreamEventKind::ToolResult` carries `is_error`, which `claude_cli_agent.rs` reads from both result shapes
    (`ClaudeCliAgent::marked_error`; a result without the mark is a success, as in the Anthropic API).
  - The attempt's live-output tap records each `ToolStep` and tool result in `LiveToolCalls`
    (`graph_task_dispatch/live_tool_calls.rs`). The attempt carries the record to `emit_feedback`, which waits for
    the tap to drain, then merges the live calls with the audited ones (the audit's outcome wins).
  - Tests: `cli_attempt_records_its_tool_calls` (fake Claude CLI, end to end),
    `efficiency_tool_calls_record_outcome_from_the_live_output`, `live_tool_calls_pair_steps_with_results`,
    `live_tool_calls_finish_waits_for_the_tap`, `event_kinds_tool_result_marked_error`.
  - Limits: only an attempt with a live-output tap records CLI calls. A tap opens for the stall watch (on by default),
    the conductor or the TUI; recording alone opens none, since a tap makes the provider stream. Outcomes need a
    trusted tap (stall watch or conductor). The provider boundary drops live events while the tap's channel is full,
    so a burst can cost a call or its outcome.
