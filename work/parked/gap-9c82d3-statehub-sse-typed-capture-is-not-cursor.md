+++
id = "gap-9c82d3"
kind = "gap"
title = "StateHub SSE typed capture is not cursor-atomic and resume has no single immutable generation"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-runtime/state-hub", "roko-serve/sse"]
created = 2026-09-15
updated = 2026-09-28
source = "gaps-md#2026-09-15-refactoring-audit-batch/statehub-cursor"
anchors = ["crates/roko-runtime/src/state_hub.rs"]
links = { depends_on = [], blocks = [], related = ["gap-8a1fb3", "gap-d40bc0"], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++

`DashboardEvent::SnapshotRebased` now advances SSE and WebSocket cursors when the snapshot is replaced (P1-04). Typed SSE capture is still not atomic with the overlay cursor, and resume does not pin one immutable snapshot generation, so a client can see an event twice or miss one across a rebase. This blocks the wider goal of the frontend DataHub, TUI, API and CLI all projecting the same durable state (remaining-work tranche 4 in GAPS.md).

Fix: capture the snapshot, overlay and cursor under one lock and generation ID, resume SSE from that generation, and test a rebase during an active stream.
