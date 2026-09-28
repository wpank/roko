+++
id = "gap-d6b2eb"
kind = "gap"
title = "[rag RAG-18 residual] Knowledge sync direction/version-vector regression tests missing"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/knowledge"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/rag-audit-2026-09-21/INDEX.md#RAG-18 — WIRED"
discovered_from = "audit:tmp/archive/rag-audit-2026-09-21/INDEX.md#RAG-18 — WIRED"
anchors = ["crates/roko-cli/src/commands/knowledge.rs", "KnowledgeSyncDirection", "crates/roko-neuro/src/sync_protocol.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Direction enum and guarded send_high_water fixes landed (bef72abc9, 01b32978b), but the regression tests specified by RAG-18 were never added to commands/knowledge.rs.

Imported without verification from:
- `tmp/archive/rag-audit-2026-09-21/INDEX.md#RAG-18 — WIRED`
- `tmp/archive/rag-audit-2026-09-21/backlog/RAG-18-knowledge-sync-validation.md`

How to verify: grep knowledge.rs tests for invalid-direction / version-vector assertions.
