+++
id = "bug-97c2dc"
kind = "bug"
title = "Async Runtime Anti-Patterns (Drop+Spawn, Unbounded Channels, Mutex Across Await)"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
subsystem = ["roko-agent"]
created = 2026-09-07
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/101-async-runtime-anti-patterns.md#101 — Async Runtime Anti-Patterns (Drop+Spawn, Unbounded Channels, Mutex…"
discovered_from = "audit:tmp/backlog/archive/101-async-runtime-anti-patterns.md#101 — Async Runtime Anti-Patterns (Drop+Spawn, Unbounded Channels, Mutex…"
anchors = ["crates/roko-agent-server/src/features/messaging.rs:171", "crates/roko-runtime/src/connector_runtime.rs:197", "crates/roko-agent/src/cursor_cli_agent.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -n 'unbounded_channel' crates/roko-agent-server/src/features/messaging.rs"
+++
latent crash and memory exhaustion risks in production dispatch paths. Tokio, the async runtime used throughout roko, has three well-known async anti-patterns that cause real failures in production:

Imported without verification from:
- `tmp/backlog/archive/101-async-runtime-anti-patterns.md#101 — Async Runtime Anti-Patterns (Drop+Spawn, Unbounded Channels, Mutex…`

How to verify: Check: No `tokio::spawn` in `Drop` impls without a `Handle::try_current()` guard.; `CursorCliAgent::Drop` does not spawn; it logs a warning if the connection was not; All three `unbounded_channel` call sites in `cursor_cli_agent.rs` replaced with [evidence: 00-STATUS-SUMMARY 3. Open / P1 -- High (Open): M | 2 |]

Verified 2026-09-28: partly fixed. The crash-class Drop+spawn part is gone, so p1 -> p2. Fixed: connector_runtime.rs:197 guards its Drop spawn with `Handle::try_current()`, and cursor_cli_agent.rs has no Drop spawn and no `unbounded_channel`. Still open: crates/roko-agent-server/src/features/messaging.rs:171 still uses `mpsc::unbounded_channel()` for the streaming event channel (acceptance item 4 of tmp/backlog/archive/101-async-runtime-anti-patterns.md). Issue 3 (mutex held across `.await`) was not re-checked.
