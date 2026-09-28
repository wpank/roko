+++
id = "bug-176b3c"
kind = "bug"
title = "De-flake process-spawning agent tests (Cursor CLI mock + ACP client) under parallelism"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/harness"]
created = 2026-09-21
updated = 2026-09-28
source = "crates/roko-agent/src/cursor_cli_agent.rs:985"
discovered_from = "audit:crates/roko-agent/src/cursor_cli_agent.rs:985"
anchors = ["crates/roko-agent/src/cursor_cli_agent.rs::cursor_cli_agent_integration_with_mock_script", "crates/roko-agent/src/harness/acp_client.rs::acp_client_full_cycle_with_mock", "crates/roko-agent/src/harness/acp_client.rs::acp_client_with_tool_notifications", "crates/roko-agent/src/harness/acp_client.rs::acp_client_session_error"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Four tests that spawn mock agent scripts are ignored as flaky under high parallelism (process resource exhaustion): Cursor CLI integration and three ACP client full-cycle/tool-notification/session-error tests.

Imported without verification from:
- `crates/roko-agent/src/cursor_cli_agent.rs:985`
- `crates/roko-agent/src/harness/acp_client.rs:1508`
- `crates/roko-agent/src/harness/acp_client.rs:1639`
- `crates/roko-agent/src/harness/acp_client.rs:1708`

How to verify: Run with --ignored --test-threads=1 vs default; consider serial_test or nextest groups instead of #[ignore].
