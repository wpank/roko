+++
id = "gap-67f436"
kind = "gap"
title = "[rag RAG-17] Conductor watchers for retrieval quality"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-conductor"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/rag-audit-2026-09-21/backlog/RAG-17-retrieval-quality-watchers.md"
discovered_from = "audit:tmp/archive/rag-audit-2026-09-21/backlog/RAG-17-retrieval-quality-watchers.md"
anchors = ["crates/roko-conductor/src/watchers/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
The 12 conductor watchers monitor ghost turns, stuck patterns, cost, context pressure, but none monitor retrieval quality from the neuro store.

Imported without verification from:
- `tmp/archive/rag-audit-2026-09-21/backlog/RAG-17-retrieval-quality-watchers.md`

How to verify: List watchers; look for a retrieval-quality watcher.
