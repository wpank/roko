+++
id = "gap-5d8325"
kind = "gap"
title = "Add MCP Discovery and Slack HTTP Mock Coverage"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-serve/channels"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/backlog/archive/356-mcp-discovery-and-slack-http-mock-coverage.md#356 — Add MCP Discovery and Slack HTTP Mock Coverage"
discovered_from = "audit:tmp/backlog/archive/356-mcp-discovery-and-slack-http-mock-coverage.md#356 — Add MCP Discovery and Slack HTTP Mock Coverage"
anchors = ["mcp/bridge.rs", "discover_mcp_runtime()", "McpRuntimeTransport"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
[blocked] Blocked on #349 — critical discovery and Slack dispatch paths have little offline coverage. The audit reported zero tests for `mcp/bridge.rs`; current source now has one narrow test, `runtime_rejects_ambiguous_server_names_before_spawning`, which covers invalid/duplicate server names…

Imported without verification from:
- `tmp/backlog/archive/356-mcp-discovery-and-slack-http-mock-coverage.md#356 — Add MCP Discovery and Slack HTTP Mock Coverage`

How to verify: Check: MCP bridge tests cover successful discovery/execution and every lifecycle/error case above.; All nine Slack tool handlers have offline request/response/error tests.; In-memory discovery transports are dropped on success/error/timeout; #314… [evidence: own status: Blocked on #349]
