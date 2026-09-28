+++
id = "gap-068922"
kind = "gap"
title = "MCP Token Savings Tracking"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-mcp-code"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/186-mcp-token-savings-tracking.md#186 — MCP Token Savings Tracking"
discovered_from = "audit:tmp/backlog/archive/186-mcp-token-savings-tracking.md#186 — MCP Token Savings Tracking"
anchors = ["roko-mcp-code/src/lib.rs", "roko-core/src/telemetry_observe.rs", ".roko/learn/mcp-savings.json", "crates/roko-mcp-code/src/lib.rs", "crates/roko-core/src/telemetry_observe.rs", "crates/roko-core/src/lib.rs", "ObservableEvent", "ObservableEventKind"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
observability; operators have no visibility into how much context the MCP tools save compared to raw file reads. The `roko-mcp-code` server provides structured code-intelligence tools (`search_code`, `get_symbol_context`, `find_references`, `get_callers`, `workspace_map`, etc.) that return…

Imported without verification from:
- `tmp/backlog/archive/186-mcp-token-savings-tracking.md#186 — MCP Token Savings Tracking`

Some cited files are gone: `.roko/learn/mcp-savings.json`.

How to verify: Check: Each successful MCP tool call increments a per-tool and cumulative token savings counter; `get_mcp_savings` tool returns the current savings summary; `ObservableEvent::McpTokenSavings` variant exists in `roko-core` [evidence: 00-STATUS-SUMMARY 3. Open / P3 -- Low (Open): XS | 7 |]
