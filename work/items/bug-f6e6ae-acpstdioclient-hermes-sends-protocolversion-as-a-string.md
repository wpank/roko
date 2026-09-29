+++
id = "bug-f6e6ae"
kind = "bug"
title = "AcpStdioClient::hermes() sends protocolVersion as a string"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
subsystem = ["roko-agent/acp-client"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-agent/src/harness/acp_client.rs::AcpStdioClient::hermes", "crates/roko-agent/src/harness/acp_client.rs::AcpStdioClient::openclaw"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = '! grep -q "protocol_version: \"2024-11-05\"" crates/roko-agent/src/harness/acp_client.rs'

[[verify]]
command = "! grep -q 'protocol_version: \"2024-11-05\"' crates/roko-agent/src/harness/acp_client.rs && cargo test -p roko-agent --lib harness::acp_client"
+++

`AcpStdioClient::hermes()` (`harness/acp_client.rs:373`) initializes with `protocol_version: "2024-11-05"` (`:379`), an MCP-style date string.
ACP's `initialize.protocolVersion` is an integer, so spec-validating ACP agents reject the handshake and this provider tier cannot connect.
Fix: send the integer ACP protocol version and cover the handshake with a strict mock agent.

Re-checked 2026-09-29: still open. AcpStdioClient::openclaw() (acp_client.rs:399) has the same "2024-11-05" string and should be fixed at the same time. The tests hermes_constructor_builds_correct_args (:1264) and the openclaw one (:1278) assert the buggy value, so the current verify command (cargo test harness::acp_client) passes while the bug is present.
