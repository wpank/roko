+++
id = "bug-f6e6ae"
kind = "bug"
title = "AcpStdioClient::hermes() sends protocolVersion as a string"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-agent/acp-client"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-agent/src/harness/acp_client.rs::AcpStdioClient::hermes"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = '! grep -q "protocol_version: \"2024-11-05\"" crates/roko-agent/src/harness/acp_client.rs'

[[verify]]
command = 'cargo test -p roko-agent --lib harness::acp_client'
+++

`AcpStdioClient::hermes()` (`harness/acp_client.rs:373`) initializes with `protocol_version: "2024-11-05"` (`:379`), an MCP-style date string.
ACP's `initialize.protocolVersion` is an integer, so spec-validating ACP agents reject the handshake and this provider tier cannot connect.
Fix: send the integer ACP protocol version and cover the handshake with a strict mock agent.
