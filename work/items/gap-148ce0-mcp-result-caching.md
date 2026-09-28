+++
id = "gap-148ce0"
kind = "gap"
title = "MCP Result Caching"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-mcp-code"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/185-mcp-result-caching.md#185 — MCP Result Caching"
discovered_from = "audit:tmp/backlog/archive/185-mcp-result-caching.md#185 — MCP Result Caching"
anchors = ["roko-mcp-code/src/lib.rs", "crates/roko-mcp-code/src/lib.rs", "dispatch_tool_call()", "WorkspaceIndex", "serde_json::Value", "load_workspace_index()", "ResultCache", "serde_json::to_string"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
performance; repeated symbol lookups across agents in the same plan run re-parse and re-index the same files. The `roko-mcp-code` server handles tool calls (`search_code`, `get_symbol_context`, `find_references`, `get_callers`, `workspace_map`, etc.) by querying a `WorkspaceIndex` loaded at…

Imported without verification from:
- `tmp/backlog/archive/185-mcp-result-caching.md#185 — MCP Result Caching`

How to verify: Check: Repeated identical tool calls within 5 minutes return cached results without re-querying the index; Cache is keyed on tool name + canonical argument JSON; LRU eviction runs every 50 calls and enforces a 500-entry cap [evidence: 00-STATUS-SUMMARY 3. Open / P3 -- Low (Open): XS | 7 |]
