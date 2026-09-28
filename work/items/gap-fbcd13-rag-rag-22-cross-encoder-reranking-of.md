+++
id = "gap-fbcd13"
kind = "gap"
title = "[rag RAG-22] Cross-encoder reranking of context candidates"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-compose"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/rag-audit-2026-09-21/backlog/RAG-22-cross-encoder-reranking.md"
discovered_from = "audit:tmp/archive/rag-audit-2026-09-21/backlog/RAG-22-cross-encoder-reranking.md"
anchors = ["crates/roko-compose/src/context_provider.rs", "crates/roko-compose/src/scorer.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
ContextBidderRegistry::propose_context candidates are ranked only by static bidder relevance and auction; add cross-encoder reranking.

Imported without verification from:
- `tmp/archive/rag-audit-2026-09-21/backlog/RAG-22-cross-encoder-reranking.md`

How to verify: grep for rerank in roko-compose.
