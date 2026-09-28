+++
id = "gap-e3d780"
kind = "gap"
title = "[rag RAG-10] Track retrieval outcomes (which injected entries helped)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-learn"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/rag-audit-2026-09-21/backlog/RAG-10-retrieval-outcome-tracking.md"
discovered_from = "audit:tmp/archive/rag-audit-2026-09-21/backlog/RAG-10-retrieval-outcome-tracking.md"
anchors = ["PromptAssemblyDiagnostics (runner/types.rs at audit time)", "crates/roko-learn/src/cascade_router.rs", "crates/roko-compose/src/auction.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Prompt assembly diagnostics record injected knowledge IDs, but no outcome attribution links retrieved entries/episodes/symbols to gate results for learning. Original anchors are Runner-v2 types.

Imported without verification from:
- `tmp/archive/rag-audit-2026-09-21/backlog/RAG-10-retrieval-outcome-tracking.md`

How to verify: Look for retrieval-outcome records joined to gate verdicts in Graph path.
