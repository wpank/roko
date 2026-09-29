+++
id = "gap-8c293b"
kind = "gap"
title = "[graph W22] OpenAPI spec coverage still incomplete (~175 of ~376 canonical routes)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-serve/openapi"]
created = 2026-09-05
updated = 2026-09-28
source = "tmp/archive/graph-audit/08-completion-status.md#all-25-work-items--done"
discovered_from = "audit:tmp/archive/graph-audit/08-completion-status.md#all-25-work-items--done"
anchors = ["crates/roko-serve/src/", "tools/http_route_inventory.py"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Audit found ~75 of ~376 canonical routes documented; fix added ~100 path entries across 12 route groups, leaving roughly half of routes undocumented.

Imported without verification from:
- `tmp/archive/graph-audit/08-completion-status.md#all-25-work-items--done`
- `tmp/archive/graph-audit/07-work-items.md#w22`

How to verify: Compare OpenAPI path count to route inventory (python3 tools/http_route_inventory.py).
