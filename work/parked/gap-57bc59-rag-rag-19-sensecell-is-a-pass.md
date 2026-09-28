+++
id = "gap-57bc59"
kind = "gap"
title = "[rag RAG-19] SenseCell is a pass-through; add RetrievalCell to cognitive graph"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-graph/cells"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/rag-audit-2026-09-21/backlog/RAG-19-retrieval-cell-cognitive-graph.md"
discovered_from = "audit:tmp/archive/rag-audit-2026-09-21/backlog/RAG-19-retrieval-cell-cognitive-graph.md"
anchors = ["crates/roko-graph/src/cells/cognitive.rs SenseCell"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Cognitive loop SenseCell returns input signals unchanged on full ticks ('A real implementation would query the Signal Store and Bus here'); add retrieval to the cognitive graph.

Imported without verification from:
- `tmp/archive/rag-audit-2026-09-21/backlog/RAG-19-retrieval-cell-cognitive-graph.md`

How to verify: Read SenseCell full-tick body.
