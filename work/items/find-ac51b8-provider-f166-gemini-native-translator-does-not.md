+++
id = "find-ac51b8"
kind = "finding"
title = "[provider F166] Gemini native translator does not sanitize MCP tool name dots"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/translate"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F166"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F166"
anchors = ["crates/roko-agent/src/translate/gemini.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`GeminiTranslator::render_tools()` maps tool names directly without calling `sanitize_tool_name()`. Dotted names (e.g., `roko-mcp-github.list_prs`) may cause issues if Gemini normalizes them differently than roko expects.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F166`
- `tmp/archive/provider-audit/21-mcp-tools.md`

How to verify: Confirm in crates/roko-agent/src/translate/gemini.rs whether still true: Gemini native translator does not sanitize MCP tool name dots
