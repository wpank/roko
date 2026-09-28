+++
id = "bug-b2dd69"
kind = "bug"
title = "Re-baseline Cursor/Codex/OpenAI parity fixtures (streaming usage + tool_call continuation drift)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/provider-parity"]
created = 2026-09-05
updated = 2026-09-28
source = "crates/roko-agent/tests/cursor_parity.rs:14"
discovered_from = "audit:crates/roko-agent/tests/cursor_parity.rs:14"
anchors = ["crates/roko-agent/tests/cursor_parity.rs::streaming", "crates/roko-agent/tests/cursor_parity.rs::tool_call", "crates/roko-agent/tests/codex_parity.rs::streaming", "crates/roko-agent/tests/codex_parity.rs::tool_call", "crates/roko-agent/tests/openai_parity.rs::streaming", "crates/roko-agent/tests/openai_parity.rs::tool_call"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
6 provider parity tests are #[ignore]d as 'pre-existing fixture drift': streaming usage mismatch and tool_call continuation mismatch for the Cursor, Codex and OpenAI backends, so the parity harness no longer proves these adapters honour the shared contract.

Imported without verification from:
- `crates/roko-agent/tests/cursor_parity.rs:14`
- `crates/roko-agent/tests/cursor_parity.rs:20`
- `crates/roko-agent/tests/codex_parity.rs:14`
- `crates/roko-agent/tests/codex_parity.rs:20`
- `crates/roko-agent/tests/openai_parity.rs:14`
- `crates/roko-agent/tests/openai_parity.rs:20`

How to verify: Run the three parity tests with --ignored; decide per failure whether fixtures or adapter usage/continuation handling are wrong.
