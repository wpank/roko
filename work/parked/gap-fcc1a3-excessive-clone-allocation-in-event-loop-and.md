+++
id = "gap-fcc1a3"
kind = "gap"
title = "Excessive Clone/Allocation in Event Loop and Dispatcher Hot Paths"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/106-memory-allocation-hot-paths.md#106 — Excessive Clone/Allocation in Event Loop and Dispatcher Hot Paths"
discovered_from = "audit:tmp/backlog/archive/106-memory-allocation-hot-paths.md#106 — Excessive Clone/Allocation in Event Loop and Dispatcher Hot Paths"
anchors = ["crates/roko-cli/", "crates/roko-agent/", "crates/roko-cli/src/runner/event_loop.rs", "crates/roko-cli/src/runner/types.rs", "crates/roko-agent/src/dispatcher/mod.rs", "crates/roko-core/src/tool/call.rs", "gate_dispatch.rs", "event_loop.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
performance; clone accumulation in gate-failure retry loops wastes heap and slows plan execution. The roko event loop (`crates/roko-cli/src/runner/event_loop.rs`, 23,154 lines) is the hot path for all plan execution. It orchestrates dispatch, gate evaluation, retry, and persistence for every task…

Imported without verification from:
- `tmp/backlog/archive/106-memory-allocation-hot-paths.md#106 — Excessive Clone/Allocation in Event Loop and Dispatcher Hot Paths`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`.

How to verify: Check: `forward_agent_events` does not call `attempt.clone()` inside the event loop body — uses `Arc::clone` or pre-computed fields.; The parallel tool dispatcher at `dispatcher/mod.rs` line 765 does not clone `call.arguments` to preserve the tool… [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): M | 6 |]
