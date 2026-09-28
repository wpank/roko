+++
id = "gap-8f1d1d"
kind = "gap"
title = "DOCS-06 D2/D3: Remove disconnected roko-mcp-slack and roko-mcp-scripts crates"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-mcp-slack"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/06-POTENTIAL-BACKLOG.md#D2. roko-mcp-slack (1,948 LOC)"
discovered_from = "audit:tmp/docs-audit/06-POTENTIAL-BACKLOG.md#D2. roko-mcp-slack (1,948 LOC)"
anchors = ["crates/roko-mcp-slack/", "crates/roko-mcp-scripts/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
roko-mcp-slack (1,948 LOC) and roko-mcp-scripts (766 LOC) were never wired to triggers/events; decision: REMOVE both.

Imported without verification from:
- `tmp/docs-audit/06-POTENTIAL-BACKLOG.md#D2. roko-mcp-slack (1,948 LOC)`
- `tmp/docs-audit/06-POTENTIAL-BACKLOG.md#D3. roko-mcp-scripts (766 LOC)`
- `tmp/dogfood/2026-09-18-session.md#Final Summary`

A source claims this was fixed; confirm against current code before closing.

Warning: every file this item cites is gone (`crates/roko-mcp-scripts/`, `crates/roko-mcp-slack/`) — likely obsolete or moved.

How to verify: Check whether crates still exist in workspace members.
