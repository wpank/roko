+++
id = "gap-511990"
kind = "gap"
title = "Agent sidecar relay bridge not implemented (--relay-url rejected; backlog #224)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent-server/relay"]
created = 2026-09-05
updated = 2026-09-28
source = "crates/roko-cli/tests/agent_serve.rs:199"
discovered_from = "audit:crates/roko-cli/tests/agent_serve.rs:199"
anchors = ["crates/roko-cli/tests/agent_serve.rs::roko_agent_serve_registers_with_relay_and_handles_messages"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
roko_agent_serve_registers_with_relay_and_handles_messages is ignored because the relay bridge is not implemented and `roko agent serve --relay-url` is rejected at startup (backlog #224).

Imported without verification from:
- `crates/roko-cli/tests/agent_serve.rs:199`

How to verify: Check agent serve startup for --relay-url rejection; cross-check E29 relay client and tmp/backlog #224.
