+++
id = "bug-6857c4"
kind = "bug"
title = "DA-bak-16: Tool advertisement mismatch (no explicit tool manifest, gemma4 8-tool cap, mcp-auto.json not discovered)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/tools"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/archive/dev-audit-2026-09-21/dev-audit-backup/16-tool-availability.md#Problem 2: `.roko/mcp.json` Does Not Exist"
discovered_from = "audit:tmp/archive/dev-audit-2026-09-21/dev-audit-backup/16-tool-availability.md#Problem 2: `.roko/mcp.json` Does Not Exist"
anchors = ["tool_selector.rs", "max_tools_before_degrade", ".roko/mcp-auto.json", "canonicalize_tool_name"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Agents hallucinated tools (list_dir, run_command, github.github__DOT_*) because prompts lacked an explicit tool manifest, gemma4 advertised 8 of 16 builtins, and MCP discovery wanted .roko/mcp.json while only mcp-auto.json existed. Stanza/alias fixes landed; the rest is unrecorded.

Imported without verification from:
- `tmp/archive/dev-audit-2026-09-21/dev-audit-backup/16-tool-availability.md#Problem 2: `.roko/mcp.json` Does Not Exist`
- `tmp/archive/dev-audit-2026-09-21/dev-audit-backup/16-tool-availability.md#Problem 5: gemma4 Tool Cap`
- `tmp/archive/dev-audit-2026-09-21/dev-audit-backup/16-tool-availability.md#Solutions`
- `tmp/archive/dev-audit-2026-09-21/dev-audit-backup/22-fix-runbook.md#P1-1: Create MCP config symlink`

How to verify: Check MCP discovery order includes mcp-auto.json and the gemma4 profile tool selection/manifest.
