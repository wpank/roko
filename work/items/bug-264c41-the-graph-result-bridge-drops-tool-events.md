+++
id = "bug-264c41"
kind = "bug"
title = "The Graph result bridge drops tool events, so Claude CLI attempts record no tool calls and tool_result errors are lost"
status = "open"
triage = "unverified"
severity = "p2"
goal = "learning"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-4d5e2d"
anchors = ["crates/roko-cli/src/graph_task_dispatch/feedback.rs", "crates/roko-agent/src/claude_cli_agent.rs"]
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
