+++
id = "find-f489db"
kind = "finding"
title = "Tool Dispatch Observability Gaps (TD-003, TD-004, TD-005)"
status = "open"
triage = "verified"
severity = "p1"
goal = "visibility"
subsystem = ["roko-agent/tool_loop"]
created = 2026-09-21
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/389-tool-dispatch-observability-gaps.md#389 — Tool Dispatch Observability Gaps (TD-003, TD-004, TD-005)"
discovered_from = "audit:tmp/backlog/archive/389-tool-dispatch-observability-gaps.md#389 — Tool Dispatch Observability Gaps (TD-003, TD-004, TD-005)"
anchors = ["crates/roko-cli/src/dispatch/factory.rs:278", "crates/roko-cli/src/dispatch_v2.rs:1519", "crates/roko-fs/src/tool_audit.rs::ToolAuditLog", "crates/roko-agent/src/tool_loop/context_factory.rs:74"]
links = { depends_on = [], blocks = [], related = ["bug-7debad"], supersedes = [], duplicate_of = "" }
+++
durable tool audit log has no production caller; timeout not enforced. Tool-audit dispatch contract findings TD-003 through TD-005 are still open: (TD-003) Durable ToolAuditLog has no production caller — only test code writes to it. (TD-004) ToolDef::timeout_ms is advertised in tool definitions…

Imported without verification from:
- `tmp/backlog/archive/389-tool-dispatch-observability-gaps.md#389 — Tool Dispatch Observability Gaps (TD-003, TD-004, TD-005)`

How to verify: Check whether the gap described in tmp/backlog/archive/389-tool-dispatch-observability-gaps.md still exists at the anchored paths. [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Audit Sweep Items (#376-#395)]

Verified 2026-09-28: TD-003 and TD-005 are still true, and TD-004 is fixed. TD-003: nothing in production opens `ToolAuditLog` or builds a `ScrubAuditAdapter` (only roko-fs tests and crates/roko-cli/tests/tool_audit_evidence.rs do), and the `with_tool_audit` setters (crates/roko-cli/src/dispatch/factory.rs:278, dispatch_v2.rs:1519) have no callers. TD-005: `ToolExecutionContextFactory` defaults to Noop audit, trace and metrics sinks (tool_loop/context_factory.rs:74-77), and the trace/metrics setters are called only in roko-core tests (tool/handler.rs:657-658). TD-004 fixed: dispatcher/mod.rs:652-660 (T029) enforces `ToolDef::timeout_ms`.
