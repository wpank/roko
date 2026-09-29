+++
id = "bug-f9ae3e"
kind = "bug"
title = "Graph efficiency records count tool events as tool calls and mark every call succeeded"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-cli/graph-dispatch", "roko-learn/efficiency"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs:1572", "crates/roko-cli/src/dispatch_v2.rs::agent_event_from_chunk", "crates/roko-learn/src/efficiency.rs::ToolCallMeta"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! sed -n '/let eff_tool_calls/,/let eff_tools_used/p' crates/roko-cli/src/graph_task_dispatch.rs | grep -q 'succeeded: true' && cargo test -p roko-cli --lib efficiency_counts_distinct_tool_call_ids"
+++

The Graph writer builds a `ToolCallMeta` for every `AgentRuntimeEvent::ToolCall` event (`graph_task_dispatch.rs:1025-1034`) with `succeeded: true` and no size or status, and sets `tools_used` to the event count (`:1043`).
Start, end and argument-delta events each produce a record, so `.roko/learn/efficiency.jsonl` over-counts tool calls and reports every call as successful with 0 bytes.
Fix: count distinct non-empty tool-call ids, take success and size from the tool audit when enabled, and otherwise mark the outcome as unobserved.

2026-09-29: re-verified at d9e79e9d8. Still open; the record is built at graph_task_dispatch.rs:1572-1611 (was ~1025). The over-count comes from dispatch_v2.rs agent_event_from_chunk, which emits a ToolCall for every ToolCallDelta. The Claude CLI adapter emits one ToolCall per tool_use block. tools_available is also set to the call count.
