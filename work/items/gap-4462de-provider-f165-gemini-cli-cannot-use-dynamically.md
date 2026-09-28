+++
id = "gap-4462de"
kind = "gap"
title = "[provider F165] Gemini CLI cannot use dynamically-discovered MCP tools"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/provider"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F165"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F165"
anchors = ["crates/roko-agent/src/provider/gemini_cli.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`GeminiCli` is excluded from `provider_supports_local_tool_runtime()`. MCP tool discovery produces definitions that would be rejected for GeminiCli. The provider has no integration path for MCP tools.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F165`
- `tmp/archive/provider-audit/21-mcp-tools.md`

How to verify: CLAUDE.md claims native Gemini CLI MCP; check dynamic MCP discovery for Gemini CLI. Confirm in crates/roko-agent/src/provider/gemini_cli.rs whether still true: Gemini CLI cannot use dynamically-discovered MCP tools
