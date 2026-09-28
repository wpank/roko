+++
id = "bug-24b970"
kind = "bug"
title = "[provider F091] McpTransportConfig::Http is config-parseable but always fails at runtime"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/mcp"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F091"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F091"
anchors = ["crates/roko-agent/src/mcp/bridge.rs", "McpTransportConfig::Http"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`McpServerConfig` parses `transport = "http"` and stores it as `McpTransportConfig::Http`. `discover_mcp_runtime()` returns `McpBridgeError::UnsupportedTransport` for any server with this transport. The config schema advertises a capability that is unimplemented.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F091`
- `tmp/archive/provider-audit/21-mcp-tools.md`

How to verify: Roadmap P4-4 (MCP HTTP/SSE transport) deferred. Check discover_mcp_runtime for Http transport. Confirm in crates/roko-agent/src/mcp/bridge.rs whether still true: `McpTransportConfig::Http` is config-parseable but always fails at runtime
