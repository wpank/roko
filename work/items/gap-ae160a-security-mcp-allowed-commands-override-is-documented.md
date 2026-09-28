+++
id = "gap-ae160a"
kind = "gap"
title = "`[security.mcp_allowed_commands]` override is documented but not wired"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/mcp"]
created = 2026-08-17
updated = 2026-09-28
source = "crates/roko-agent/src/mcp/config.rs:123"
discovered_from = "audit:crates/roko-agent/src/mcp/config.rs:123"
anchors = ["crates/roko-agent/src/mcp/config.rs::DEFAULT_ALLOWED_COMMANDS"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
DEFAULT_ALLOWED_COMMANDS doc comment says operators can extend the MCP server command allowlist via [security.mcp_allowed_commands] in roko.toml '(not yet wired)'. Doctor warnings cannot be tuned and the security config key is silently ignored.

Imported without verification from:
- `crates/roko-agent/src/mcp/config.rs:123`

How to verify: grep roko-core config schema for mcp_allowed_commands; check doctor MCP command check.
