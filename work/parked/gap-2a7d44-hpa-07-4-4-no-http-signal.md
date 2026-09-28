+++
id = "gap-2a7d44"
kind = "gap"
title = "HPA-07 §4.4: No HTTP signal list/lookup endpoints for signal DAG replay"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-serve/routes"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/hermes-product-audit/07-roko-ux-roadmap.md#4.4 Signal DAG replay"
discovered_from = "audit:tmp/hermes-product-audit/07-roko-ux-roadmap.md#4.4 Signal DAG replay"
anchors = ["crates/roko-serve/src/routes/", "roko replay"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Web replay needs GET /api/signals (paginated) and GET /api/signals/:hash reading the FileSubstrate; listed as one of three items needing new backend code.

Imported without verification from:
- `tmp/hermes-product-audit/07-roko-ux-roadmap.md#4.4 Signal DAG replay`
- `tmp/hermes-product-audit/07-roko-ux-roadmap.md#What genuinely needs new backend work`

How to verify: grep routes for /api/signals.
