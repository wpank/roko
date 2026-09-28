+++
id = "bug-7debad"
kind = "bug"
title = "Tool audit P0s: production ToolLoopAgent uses ToolContext::testing; cancellation/timeout/audit gaps"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-agent/tool-loop"]
created = 2026-09-04
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#3. Tool Audit (`tmp/tool-audit/`, 13 files, 36 findings)"
discovered_from = "audit:tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#3. Tool Audit (`tmp/tool-audit/`, 13 files, 36 findings)"
anchors = ["crates/roko-agent/src/tool_loop/mod.rs:1266", "crates/roko-agent/src/tool_loop/context_factory.rs::ToolExecutionContextFactory", "crates/roko-cli/src/dispatch/factory.rs:278"]
links = { depends_on = [], blocks = [], related = ["find-f489db"], supersedes = [], duplicate_of = "" }
+++
TD-001 production ToolLoopAgent uses ToolContext::testing (broad test capabilities); TD-002 cancellation not propagated to tools; TD-003 audit log unwired; TD-004 per-tool timeout ignored; TD-006 parallel result miscorrelation.

Imported without verification from:
- `tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#3. Tool Audit (`tmp/tool-audit/`, 13 files, 36 findings)`
- `tmp/tool-audit/`

How to verify: grep for ToolContext::testing outside #[cfg(test)].

Verified 2026-09-28: partly fixed. The remaining scope is lower severity, so p1 -> p2. Fixed: TD-001, since `ToolLoopAgent` builds contexts through `ToolExecutionContextFactory` -> `ToolContext::production` (crates/roko-agent/src/tool_loop/agent_wrapper.rs:213, context_factory.rs:151), and `ToolContext::testing` now appears only under `#[cfg(test)]` and in testutil.rs. TD-002, since providers pass their cancel token into the agent (e.g. provider/anthropic_api/tool_loop.rs:86) and agent_wrapper.rs:202 threads it into every context. TD-004, since dispatcher/mod.rs:652-660 (T029) enforces `ToolDef::timeout_ms`. Still open: TD-003, the durable audit log has no production writer (tracked in find-f489db). TD-006, crates/roko-agent/src/tool_loop/mod.rs:1266-1280 stores `current_calls` in provider order beside previews built from `dispatch_batch` results, so traces still join them by vector index (current `dispatch_batch` ordering not re-checked).
