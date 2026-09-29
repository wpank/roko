+++
id = "gap-b7258c"
kind = "gap"
title = "GET /api/knowledge/search is a stub that always returns no results"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["roko-serve/knowledge"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/routes/aggregator.rs::search_knowledge"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -A3 'async fn search_knowledge' crates/roko-serve/src/routes/aggregator.rs | grep -q 'State(_state)' && cargo test -p roko-serve knowledge_search_returns_matching_entries"
+++

`search_knowledge` (`routes/aggregator.rs:584`) returns `{"results": [], "query": q, "timestamp"}` unconditionally.
It is the only knowledge search route a read-scope key can call (`POST /api/neuro/query` needs write scope), so read-only clients and dashboards always see an empty knowledge base.
Fix: back it with the knowledge store query (same scoring as `/api/neuro/query`), bounded and read-only.

Re-checked 2026-09-29: unchanged (stub at crates/roko-serve/src/routes/aggregator.rs:584-593). The existing verify is unsound: no aggregator test calls /knowledge/search, so it passes with the stub in place.
