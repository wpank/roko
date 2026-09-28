+++
id = "find-98130a"
kind = "finding"
title = "HTTP API Response Envelope and Pagination Inconsistencies"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-serve"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/105-http-api-design-consistency.md#105 — HTTP API Response Envelope and Pagination Inconsistencies"
discovered_from = "audit:tmp/backlog/archive/105-http-api-design-consistency.md#105 — HTTP API Response Envelope and Pagination Inconsistencies"
anchors = ["crates/roko-serve/", "routes/agents.rs", "routes/aggregator.rs", "routes/auth.rs", "routes/feeds.rs", "routes/arenas.rs", "routes/mod.rs", "episodes.jsonl"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
API quality; inconsistent envelopes break client code that reads list endpoints. The roko HTTP control plane (`roko-serve`) exposes roughly 317 routes registered in `crates/roko-serve/src/routes/`. These routes were built incrementally across many epics without a consistent API design standard…

Imported without verification from:
- `tmp/backlog/archive/105-http-api-design-consistency.md#105 — HTTP API Response Envelope and Pagination Inconsistencies`

How to verify: Check: All five list endpoints (`managed-agents`, `api-keys`, aggregator predictions/sessions, aggregator predictions/claims, feeds) return JSON in the shape `{"data": [...], "total": N, "offset": N, "limit": N, "has_more": bool}`.; All five… [evidence: own status: Historical packet requiring current-source revalidation; it is not scheduled by the CLI-audit checklist…; 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): M…]
