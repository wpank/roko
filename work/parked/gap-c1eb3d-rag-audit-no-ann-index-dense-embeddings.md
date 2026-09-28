+++
id = "gap-c1eb3d"
kind = "gap"
title = "RAG audit: no ANN index, dense embeddings unwired, no reranking/write-back, code index outside prompt budget"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-neuro+roko-index"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#9. RAG Audit (`tmp/rag-audit/`, 1 file)"
discovered_from = "audit:tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#9. RAG Audit (`tmp/rag-audit/`, 1 file)"
anchors = ["roko-neuro retrieval", "roko-index"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
~70% of RAG exists but scattered: missing ANN (HNSW/LSH) index, dense embeddings not wired to the index, cross-encoder reranking, verified write-back loop, code index in the prompt budget system, per-role context scoping.

Imported without verification from:
- `tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#9. RAG Audit (`tmp/rag-audit/`, 1 file)`
- `tmp/rag-audit/`

How to verify: Check roko-neuro/roko-index for ANN/embedding code.
