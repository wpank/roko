+++
id = "bug-f9ae3e"
kind = "bug"
title = "Graph efficiency records count tool events as tool calls and mark every call succeeded"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-cli/graph-dispatch", "roko-learn/efficiency"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs:1025"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

The Graph writer builds a `ToolCallMeta` for every `AgentRuntimeEvent::ToolCall` event (`graph_task_dispatch.rs:1025-1034`) with `succeeded: true` and no size or status, and sets `tools_used` to the event count (`:1043`).
Start, end and argument-delta events each produce a record, so `.roko/learn/efficiency.jsonl` over-counts tool calls and reports every call as successful with 0 bytes.
Fix: count distinct non-empty tool-call ids, take success and size from the tool audit when enabled, and otherwise mark the outcome as unobserved.
