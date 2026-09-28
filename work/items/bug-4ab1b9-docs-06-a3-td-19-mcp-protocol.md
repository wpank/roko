+++
id = "bug-4ab1b9"
kind = "bug"
title = "DOCS-06 A3 / TD-19: MCP protocol version mismatch between server and client"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-mcp-stdio"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/06-POTENTIAL-BACKLOG.md#A3. MCP protocol version mismatch"
discovered_from = "audit:tmp/docs-audit/06-POTENTIAL-BACKLOG.md#A3. MCP protocol version mismatch"
anchors = ["crates/roko-mcp-stdio/", "protocolVersion", "crates/roko-mcp-code", "crates/roko-agent/src/mcp/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Roko MCP servers advertise protocol 2024-11-05 while the client sends 2025-11-25, risking negotiation failures with strict peers (decision: align).

Imported without verification from:
- `tmp/docs-audit/06-POTENTIAL-BACKLOG.md#A3. MCP protocol version mismatch`
- `tmp/docs-audit/07-TECH-DEBT.md#TD-19: MCP Protocol Version Alignment`
- `tmp/dogfood/2026-09-18-session.md#Final Summary`
- `docs/v3/39-ROADMAP.md#3.3 Half-Implemented Features`

A source claims this was fixed; confirm against current code before closing.

How to verify: grep protocolVersion constants in roko-mcp-* and roko-agent MCP client. / Locate the scaffolding named in the roadmap row and confirm no runtime caller.

Merged 2 mined candidates: m4-033, m5-077.
