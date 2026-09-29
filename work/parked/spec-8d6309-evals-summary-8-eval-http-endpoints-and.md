+++
id = "spec-8d6309"
kind = "spec"
title = "[evals SUMMARY #8] Eval HTTP endpoints and SSE stream missing"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-serve/routes"]
created = 2026-09-02
updated = 2026-09-28
source = "tmp/archive/evals-audit/SUMMARY.md#Recommended Priority Order"
discovered_from = "audit:tmp/archive/evals-audit/SUMMARY.md#Recommended Priority Order"
anchors = ["crates/roko-serve/src/routes/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Expose GET /api/evals/summary, /api/evals/rungs, /api/evals/history backed by arena + gate verdict store, plus an SSE stream for live eval events.

Imported without verification from:
- `tmp/archive/evals-audit/SUMMARY.md#Recommended Priority Order`
- `tmp/archive/evals-audit/16-endpoint-data-flow.md`

How to verify: grep roko-serve routes for /api/evals.
