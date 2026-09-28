+++
id = "gap-af8aba"
kind = "gap"
title = "[provider F092] No persistent MCP server pool — new process spawned per agent dispatch"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/mcp"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F092"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F092"
anchors = ["crates/roko-agent/src/mcp/bridge.rs", "crates/roko-cli/src/dispatch_v2.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Each call to `discover_mcp_runtime()` spawns fresh child processes for every configured MCP server. For a plan with N tasks using GitHub MCP tools, the `roko-mcp-github` server is started and stopped N times.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F092`
- `tmp/archive/provider-audit/21-mcp-tools.md`

How to verify: Confirm in crates/roko-agent/src/mcp/bridge.rs, crates/roko-cli/src/dispatch_v2.rs whether still true: No persistent MCP server pool — new process spawned per agent dispatch
