+++
id = "gap-d25042"
kind = "gap"
title = "Knowledge Write-Back Proof (End-to-End Neuro Store Verification)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/runner"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/142-knowledge-write-back-proof.md#142 — Knowledge Write-Back Proof (End-to-End Neuro Store Verification)"
discovered_from = "audit:tmp/backlog/archive/142-knowledge-write-back-proof.md#142 — Knowledge Write-Back Proof (End-to-End Neuro Store Verification)"
anchors = ["crates/roko-cli/src/runner/event_loop.rs", "crates/roko-neuro/src/", "crates/roko-serve/src/routes/", ".roko/neuro/knowledge.jsonl", "tests/knowledge_proof/write_back.sh", ".roko/episodes.jsonl", "crates/roko-compose/src/", "tests/knowledge_proof/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The knowledge write-back path (`RuntimeKnowledgeLifecycle::ingest_episode`) is called from the runner but has not been verified end-to-end; without proof, `roko knowledge query` may return empty results even after successful runs.. The self-improving loop depends on knowledge accumulated from…

Imported without verification from:
- `tmp/backlog/archive/142-knowledge-write-back-proof.md#142 — Knowledge Write-Back Proof (End-to-End Neuro Store Verification)`
- `tmp/backlog/_archive/_mori-diffs-gaps.md#§F-3 (suggested 126)`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`, `tests/knowledge_proof/`, `tests/knowledge_proof/write_back.sh`.

How to verify: Check: After run 1, `.roko/neuro/knowledge.jsonl` has at least one valid entry.; `roko knowledge query "<task-topic>"` returns the entry from run 1.; `GET /api/neuro/query?q=<topic>` returns the same entry. [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): S | 5 |]
