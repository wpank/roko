+++
id = "spec-49b665"
kind = "spec"
title = "MCP Protocol Modernization for Editor Integration"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-mcp-code"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/435-mcp-protocol-modernization-editor-integration.md#435 — MCP Protocol Modernization for Editor Integration"
discovered_from = "audit:tmp/backlog/archive/435-mcp-protocol-modernization-editor-integration.md#435 — MCP Protocol Modernization for Editor Integration"
anchors = ["crates/roko-mcp-code/", "crates/roko-mcp-stdio/", "crates/roko-core/", "server/discover", "tools/list", "application/json", "resources/list", "resources/read"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
roko-mcp-code uses pre-2026-07-28 MCP spec; missing Tasks, Resources, Prompts, outputSchema. The MCP (Model Context Protocol) spec released version 2026-07-28 on July 28, 2026, with major architectural changes. Roko's `roko-mcp-code` crate implements a stdio MCP server with 15 code intelligence…

Imported without verification from:
- `tmp/backlog/archive/435-mcp-protocol-modernization-editor-integration.md#435 — MCP Protocol Modernization for Editor Integration`

Some cited files are gone: `application/json`, `resources/list`, `resources/read`, `server/discover`, `tools/list`.

How to verify: Check: `roko-mcp-code` accepts requests without `initialize` handshake; `server/discover` returns capabilities; `tools/list` includes `ttlMs` and `cacheScope` [evidence: 00-INDEX (2026-09-21) listed active: ACP v2, Editor UX, and MCP Modernization (#18, #39]
