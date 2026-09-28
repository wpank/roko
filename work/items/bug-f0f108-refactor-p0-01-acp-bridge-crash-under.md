+++
id = "bug-f0f108"
kind = "bug"
title = "ACP bridge crash under sustained load (analyzed, not fixed)"
status = "open"
triage = "verified"
severity = "p1"
goal = "hermes"
subsystem = ["roko-acp/bridge_events"]
created = 2026-09-15
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p0-critical-data-loss-crashes-correctness"
discovered_from = "audit:tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p0-critical-data-loss-crashes-correctness"
anchors = ["crates/roko-acp/src/acp_adapter.rs:191", "crates/roko-acp/src/bridge_events/permissions.rs:260", "crates/roko-acp/src/bridge_events/mod.rs:551"]
links = { depends_on = [], blocks = [], related = ["bug-c59522", "bug-b2a9de"], supersedes = [], duplicate_of = "" }
+++
Analysis found 5 crash-risk categories (2 potentially P0-grade): unreachable! in builtin_tools.rs, bounded channel exhaustion without backpressure, lock poisoning, SSE channel lag, WebSocket backpressure. No source changes were made.

Imported without verification from:
- `tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p0-critical-data-loss-crashes-correctness`
- `tmp/archive/refactoring-audit-2026-09-21/P0-01-ACP-CRASH-ANALYSIS.md`

Warning: every file this item cites is gone (`crates/roko-acp/src/bridge_events.rs`) — likely obsolete or moved.

How to verify: Check roko-acp for the unreachable! in builtin_tools.rs, bounded channel send behavior under load, Mutex poisoning handling, SSE lag handling; related provider F005.

Verified 2026-09-28: the crash sites are gone, so p0 -> p1. builtin_tools.rs:449 replaced the `unreachable!`, there is no production unwrap/expect, and poisoned locks are recovered. Still open from P0-01's recommendations: P0-A, where `AcpAdapter::consume` (crates/roko-acp/src/acp_adapter.rs:189-194) still `try_send`s every event, including `WorkflowCompleted`, and only warns when the channel is full. P1-B, where bridge_events/permissions.rs:260 still polls every 25ms. Backpressure between the provider stream and the editor (3a) and the single-threaded handler loop (4a, handler.rs:141) were not re-checked.
