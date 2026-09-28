+++
id = "gap-a0326b"
kind = "gap"
title = "[rag RAG-04] `[retrieval]` config section (RetrievalConfig) — conflicting done/open status"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core/config"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/rag-audit-2026-09-21/INDEX.md#RAG-04 is the sole open Phase 1 item"
discovered_from = "audit:tmp/archive/rag-audit-2026-09-21/INDEX.md#RAG-04 is the sole open Phase 1 item"
anchors = ["crates/roko-core/src/config/retrieval.rs", "RokoConfig.retrieval", "RetrievalConfig"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
backlog/INDEX marks RAG-04 DONE 2026-09-14 (RetrievalConfig: mode, max_results, min_score, token_budget...), but top-level INDEX.md (same date) says no RetrievalConfig/retrieval.rs/RokoConfig.retrieval exists. Blocks RAG-05/08/12/15/22.

Imported without verification from:
- `tmp/archive/rag-audit-2026-09-21/INDEX.md#RAG-04 is the sole open Phase 1 item`
- `tmp/archive/rag-audit-2026-09-21/backlog/INDEX.md#Phase 1: Verify and Fix (Week 1-2)`

A source claims this was fixed; confirm against current code before closing.

How to verify: grep -rn 'RetrievalConfig' crates/roko-core; check whether any retrieval path reads it.
