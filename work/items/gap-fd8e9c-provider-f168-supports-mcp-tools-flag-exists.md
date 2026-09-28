+++
id = "gap-fd8e9c"
kind = "gap"
title = "[provider F168] supports_mcp_tools flag exists but not enforced in tool filtering"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/provider"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F168"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F168"
anchors = ["crates/roko-agent/src/provider/openai_compat.rs", "supports_mcp_tools"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`ModelCapabilities.supports_mcp_tools` is advertised via ACP but does not gate which tools are passed in `tool_registry_for_options`. A model with `supports_mcp_tools: false` still receives MCP tool definitions.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F168`
- `tmp/archive/provider-audit/21-mcp-tools.md`

How to verify: Confirm in crates/roko-agent/src/provider/openai_compat.rs whether still true: `supports_mcp_tools` flag exists but not enforced in tool filtering
