+++
id = "gap-97d5d4"
kind = "gap"
title = "[rag RAG-07] Add HNSW ANN index for HDC similarity search"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-primitives/hdc"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/rag-audit-2026-09-21/backlog/RAG-07-hnsw-index.md"
discovered_from = "audit:tmp/archive/rag-audit-2026-09-21/backlog/RAG-07-hnsw-index.md"
anchors = ["crates/roko-primitives/src/hdc.rs", "crates/roko-primitives/src/codebook.rs", "crates/roko-neuro/src/knowledge_store.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
All HDC similarity searches are O(n) linear scans (~500us at 10K, >5ms at 100K+), too slow for synchronous query_hdc/query_similar_episodes on the dispatch hot path.

Imported without verification from:
- `tmp/archive/rag-audit-2026-09-21/backlog/RAG-07-hnsw-index.md`

Some cited files are gone: `crates/roko-neuro/src/knowledge_store.rs`.

How to verify: grep for hnsw/ANN index in roko-primitives/roko-neuro.
