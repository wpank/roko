+++
id = "gap-33f7e3"
kind = "gap"
title = "Add `retrieve` Built-in Tool to roko-std"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-std/tool"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/377-rag-retrieve-builtin-tool.md#377 — Add `retrieve` Built-in Tool to roko-std"
discovered_from = "audit:tmp/backlog/archive/377-rag-retrieve-builtin-tool.md#377 — Add `retrieve` Built-in Tool to roko-std"
anchors = ["crates/roko-std/src/tool/builtin/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
agents cannot directly query knowledge store mid-turn. The RAG pipeline is 95% complete (21/22 items wired including HNSW, dense embeddings, UnifiedRetrievalContextBidder, RetrievalCell, reranker, A/B experiments, foraging). The only missing piece is a standalone `retrieve` tool that agents can…

Imported without verification from:
- `tmp/backlog/archive/377-rag-retrieve-builtin-tool.md#377 — Add `retrieve` Built-in Tool to roko-std`

How to verify: Check whether the gap described in tmp/backlog/archive/377-rag-retrieve-builtin-tool.md still exists at the anchored paths. [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Audit Sweep Items (#376-#395)]
