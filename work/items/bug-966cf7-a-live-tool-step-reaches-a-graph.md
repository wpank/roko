+++
id = "bug-966cf7"
kind = "bug"
title = "A live tool step reaches a Graph run's event index only when a later row arrives at least 1 s after the last flush, up to ~5.5 s late"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-14
updated = 2026-10-14
last_verified = 2026-10-14
source = "roko-loops worker for the hermes-plugin-roko live UI, 2026-10-14 (roko main 216f88c84)"
anchors = ["crates/roko-cli/src/graph_execution/event_log.rs::WorkspaceLogWriter"]
links = { depends_on = [], blocks = [], related = ["bug-0b997a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn live_records_flush_at_once' crates/roko-cli/ && cargo test -p roko-cli live_records_flush_at_once"
+++

## Problem

A Graph run's event index (`WorkspaceEventLog`) flushes lifecycle rows at once, and other rows only when a later row
arrives at least 1 s after the last flush (bug-0b997a). A live tool step published during an agent's provider call is
therefore held until the next heartbeat (every 5 s), so the per-run SSE (`/api/runs/{id}/events/stream`) and
`GET /api/runs/{id}/events` show it up to about 5.5 s late; so are route decisions, confirm waits, held effects,
knowledge writes, diagnoses and gate rung starts.

## Why it matters

A host following a live run sees the agent's steps in bursts instead of as they happen (goal `visibility`).

## Where

`crates/roko-cli/src/graph_execution/event_log.rs::WorkspaceLogWriter::record` and `is_lifecycle_event`.

## Current state

Heartbeats and efficiency rows benefit from the batching; nothing else needs it.

## Plan

Flush the index at once (with relaxed durability) for live agent output records and the loop events a host shows as
they happen; keep batching heartbeats, efficiency rows and other high-volume rows.

## Done when

`[[verify]]` passes: a live record is in the run's index when it has been recorded, without a later row.
