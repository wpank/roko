+++
id = "gap-db3eff"
kind = "gap"
title = "HPA-04 §7.9: POST /api/inference/complete has no streaming variant"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-serve/gateway"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.9. No streaming for `POST /api/inference/complete`"
discovered_from = "audit:tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.9. No streaming for `POST /api/inference/complete`"
anchors = ["/api/inference/complete"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
The gateway streams internally but the HTTP endpoint returns only after completion, so remote one-shot inference cannot render incrementally.

Imported without verification from:
- `tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.9. No streaming for `POST /api/inference/complete``

Warning: every file this item cites is gone (`/api/inference/complete`) — likely obsolete or moved.

How to verify: Check inference routes for SSE/stream support.
