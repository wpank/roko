+++
id = "bug-309002"
kind = "bug"
title = "[engine row10-statehub-cursor] StateHub snapshot/SSE cursor atomicity marked live, but CLAUDE.md lists it as partial"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-runtime/state_hub"]
created = 2026-09-05
updated = 2026-09-28
source = "tmp/archive/engine-audit/DEFERRED-SPEC-LEDGER.md#disposition-table"
discovered_from = "audit:tmp/archive/engine-audit/DEFERRED-SPEC-LEDGER.md#disposition-table"
anchors = ["crates/roko-runtime/src/state_hub.rs", "StateHub::cursor_snapshot", "StateHub::subscribe_events_from", "crates/roko-serve/src/routes/sse.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Deferred ledger row 10 says already live via #248 (StateHub::cursor_snapshot, subscribe_events_from, SSE gap handling), while CLAUDE.md still states 'StateHub overlay/SSE cursor atomicity and single-generation resume remain explicit partial work'.

Imported without verification from:
- `tmp/archive/engine-audit/DEFERRED-SPEC-LEDGER.md#disposition-table`

A source claims this was fixed; confirm against current code before closing.

How to verify: Check overlay+cursor snapshot atomicity for SSE resume and single-generation resume in state_hub.rs/sse.rs; reconcile with CLAUDE.md.
