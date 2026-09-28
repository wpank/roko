+++
id = "gap-0b7082"
kind = "gap"
title = "[provider F103] McpCapabilities always reports {http: true, sse: true} regardless of actual support"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-acp/handler"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F103"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F103"
anchors = ["crates/roko-acp/src/handler.rs", "McpCapabilities", "{http: true, sse: true}"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
The ACP `initialize` response's `mcpCapabilities` field is hardcoded to `{http: true, sse: true}`. Only stdio MCP transport is actually implemented. ACP clients may attempt HTTP/SSE MCP transports that will fail.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F103`
- `tmp/archive/provider-audit/04-acp-integration.md`

How to verify: Related to deferred P4-4; CLAUDE.md claims ACP truthful capabilities. Check McpCapabilities reporting. Confirm in crates/roko-acp/src/handler.rs whether still true: `McpCapabilities` always reports `{http: true, sse: true}` regardless of actual support
