+++
id = "gap-754ab5"
kind = "gap"
title = "HPA-04 §7.1/R1: Remote clients face three event schemas (DashboardEvent, ServerEvent, RuntimeEvent)"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-serve/events"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.1. No unified \"what happened\" event type — two overlapping schemas"
discovered_from = "audit:tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.1. No unified \"what happened\" event type — two overlapping schemas"
anchors = ["crates/roko-serve/src/events.rs", "crates/roko-core/src/dashboard_snapshot.rs", "crates/roko-serve/src/routes/ws.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
SSE /api/events, WS /ws and per-run SSE emit three different enums; WS-only events (AgentTrace, Inference*, SomaticMarkerFired, TriggerFired) never reach SSE and HTTP-run ServerEvents are only loosely projected into StateHub. Proposed ClientEvent wire format. CoreEvent bridge (P1-EVT-1) landed 09...

Imported without verification from:
- `tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.1. No unified "what happened" event type — two overlapping schemas`
- `tmp/hermes-product-audit/04-roko-client-server-gaps.md#R1 (High Priority): Unify the event schema for remote clients`
- `tmp/hermes-product-audit/03-roko-streaming-architecture.md#6. Two-Bus Architecture: The Gap`
- `tmp/dogfood/2026-09-20-final-session.md#P1 (High)`

How to verify: Compare event types emitted on /ws vs /api/events for one run.
