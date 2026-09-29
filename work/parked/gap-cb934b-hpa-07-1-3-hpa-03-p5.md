+++
id = "gap-cb934b"
kind = "gap"
title = "HPA-07 §1.3 / HPA-03 P5: No atomic snapshot+cursor bootstrap for web clients"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-serve/statehub"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/hermes-product-audit/07-roko-ux-roadmap.md#1.3 Snapshot bootstrap endpoint"
discovered_from = "audit:tmp/hermes-product-audit/07-roko-ux-roadmap.md#1.3 Snapshot bootstrap endpoint"
anchors = ["crates/roko-runtime/src/state_hub.rs", "/api/statehub/snapshot"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Proposed GET /api/snapshot returning {snapshot, cursor} (next_seq) so clients resume SSE exactly after the snapshot; snapshot currently lacks a sequence number.

Imported without verification from:
- `tmp/hermes-product-audit/07-roko-ux-roadmap.md#1.3 Snapshot bootstrap endpoint`
- `tmp/hermes-product-audit/03-roko-streaming-architecture.md#Priority 5: Add Seq Number to Snapshot`

Some cited files are gone: `/api/statehub/snapshot`.

How to verify: Check snapshot response for a cursor/seq field.
