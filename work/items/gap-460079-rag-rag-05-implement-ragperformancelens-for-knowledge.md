+++
id = "gap-460079"
kind = "gap"
title = "[rag RAG-05] Implement RagPerformanceLens for knowledge-store retrieval telemetry"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-neuro/telemetry"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/rag-audit-2026-09-21/backlog/RAG-05-rag-performance-lens.md"
discovered_from = "audit:tmp/archive/rag-audit-2026-09-21/backlog/RAG-05-rag-performance-lens.md"
anchors = ["crates/roko-core/src/telemetry_observe.rs", "KnowledgeStore::query"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
KnowledgeStore::query/query_similar/query_hits emit no latency, result-count or cache metrics into the Lens pipeline; TUI F10/F5 show no retrieval numbers. Blocks RAG-06/14/16/17.

Imported without verification from:
- `tmp/archive/rag-audit-2026-09-21/backlog/RAG-05-rag-performance-lens.md`
- `tmp/archive/rag-audit-2026-09-21/backlog/INDEX.md#Phase 2: Metrics and Visualization (Week 3-4)`

How to verify: grep for RagPerformance / rag_performance lens registration.
