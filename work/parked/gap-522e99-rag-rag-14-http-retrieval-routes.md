+++
id = "gap-522e99"
kind = "gap"
title = "[rag RAG-14] HTTP retrieval routes"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-serve/routes"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/rag-audit-2026-09-21/backlog/RAG-14-retrieval-http-routes.md"
discovered_from = "audit:tmp/archive/rag-audit-2026-09-21/backlog/RAG-14-retrieval-http-routes.md"
anchors = ["crates/roko-serve/src/routes/retrieval.rs (proposed)", "GET /api/projections/rag_performance"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Retrieval events are only in-process (Lens/StateHub); add structured HTTP retrieval routes beyond the raw rag_performance projection.

Imported without verification from:
- `tmp/archive/rag-audit-2026-09-21/backlog/RAG-14-retrieval-http-routes.md`

Warning: every file this item cites is gone (`crates/roko-serve/src/routes/retrieval.rs`) — likely obsolete or moved.

How to verify: grep roko-serve routes for retrieval.
