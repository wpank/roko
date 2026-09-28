+++
id = "bug-82d170"
kind = "bug"
title = "[rag RAG-13] WorkspaceIndex never refreshed within a session"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-index"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/rag-audit-2026-09-21/backlog/RAG-13-index-refresh-trigger.md"
discovered_from = "audit:tmp/archive/rag-audit-2026-09-21/backlog/RAG-13-index-refresh-trigger.md"
anchors = ["crates/roko-index/src/workspace.rs WorkspaceIndex", "crates/roko-mcp-code/src/lib.rs", "runner/impact_analysis.rs (at audit time)"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
WorkspaceIndex is built once at startup; file changes by agents or the developer are invisible to later code-intelligence calls (stale impact analysis / MCP code results).

Imported without verification from:
- `tmp/archive/rag-audit-2026-09-21/backlog/RAG-13-index-refresh-trigger.md`

How to verify: Check for file-watch/refresh/invalidate on WorkspaceIndex.
