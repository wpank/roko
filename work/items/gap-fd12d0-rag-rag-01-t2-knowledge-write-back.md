+++
id = "gap-fd12d0"
kind = "gap"
title = "[rag RAG-01 T2] Knowledge write-back entries too thin to match topic queries"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-neuro"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/rag-audit-2026-09-21/INDEX.md#RAG-01 — WIRED"
discovered_from = "audit:tmp/archive/rag-audit-2026-09-21/INDEX.md#RAG-01 — WIRED"
anchors = ["NeuroKnowledgeIngestor", "KnowledgeIngestionSink", "crates/roko-neuro/src/admission.rs KnowledgeAdmission", "crates/roko-cli/src/serve_runtime.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
RAG-01 is wired, but NeuroKnowledgeIngestor writes entries like 'task X completed via model@provider in Nms' that rarely match topic queries; enrich with task title + agent output (RAG-01 T2 residual).

Imported without verification from:
- `tmp/archive/rag-audit-2026-09-21/INDEX.md#RAG-01 — WIRED`
- `tmp/archive/rag-audit-2026-09-21/backlog/RAG-01-verify-knowledge-writeback.md`

How to verify: Inspect NeuroKnowledgeIngestor entry content; sample .roko/neuro/knowledge.jsonl entries.
