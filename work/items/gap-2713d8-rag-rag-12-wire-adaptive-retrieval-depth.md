+++
id = "gap-2713d8"
kind = "gap"
title = "[rag RAG-12] Wire adaptive retrieval depth (MultiPatchForager MVT)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-compose/foraging"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/rag-audit-2026-09-21/backlog/RAG-12-adaptive-retrieval-depth.md"
discovered_from = "audit:tmp/archive/rag-audit-2026-09-21/backlog/RAG-12-adaptive-retrieval-depth.md"
anchors = ["crates/roko-compose/src/foraging.rs MultiPatchForager", "crates/roko-neuro/src/context.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
MultiPatchForager implements MVT stopping (optimal_iterations, should_visit, social_foraging_boost) but none are called from the retrieval pipeline.

Imported without verification from:
- `tmp/archive/rag-audit-2026-09-21/backlog/RAG-12-adaptive-retrieval-depth.md`

How to verify: grep optimal_iterations/should_visit call sites outside tests.
