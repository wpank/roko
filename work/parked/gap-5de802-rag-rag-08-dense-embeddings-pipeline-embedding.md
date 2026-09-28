+++
id = "gap-5de802"
kind = "gap"
title = "[rag RAG-08] Dense embeddings pipeline: embedding agents' output discarded"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-index"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/rag-audit-2026-09-21/backlog/RAG-08-dense-embeddings-pipeline.md"
discovered_from = "audit:tmp/archive/rag-audit-2026-09-21/backlog/RAG-08-dense-embeddings-pipeline.md"
anchors = ["crates/roko-agent/src/gemini/embed.rs", "crates/roko-agent/src/perplexity/embed.rs", "crates/roko-index/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Gemini and Perplexity embed agents produce Vec<Vec<f32>> from live API calls but nothing persists or queries them; the one dense-embedding search method silently falls back to keyword search.

Imported without verification from:
- `tmp/archive/rag-audit-2026-09-21/backlog/RAG-08-dense-embeddings-pipeline.md`

How to verify: Find callers of the embed agents and the dense search fallback.
