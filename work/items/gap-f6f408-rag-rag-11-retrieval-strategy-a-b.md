+++
id = "gap-f6f408"
kind = "gap"
title = "[rag RAG-11] Retrieval strategy A/B experiments"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-learn/prompt_experiment"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/rag-audit-2026-09-21/backlog/RAG-11-retrieval-ab-experiments.md"
discovered_from = "audit:tmp/archive/rag-audit-2026-09-21/backlog/RAG-11-retrieval-ab-experiments.md"
anchors = ["crates/roko-learn/src/prompt_experiment.rs", "runner/prompt_experiments.rs (at audit time)"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Keyword-only, HDC-augmented and hybrid-with-episode-fusion retrieval paths are never compared; wire them into the prompt experiment framework to measure gate pass rate impact.

Imported without verification from:
- `tmp/archive/rag-audit-2026-09-21/backlog/RAG-11-retrieval-ab-experiments.md`

Some cited files are gone: `runner/prompt_experiments.rs`.

How to verify: grep experiments for retrieval strategy arms.
