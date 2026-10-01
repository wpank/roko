+++
id = "gap-b7258c"
kind = "gap"
title = "GET /api/knowledge/search is a stub that always returns no results"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["roko-serve/knowledge"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/routes/aggregator.rs::search_knowledge"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -A3 'async fn search_knowledge' crates/roko-serve/src/routes/aggregator.rs | grep -q 'State(_state)' && grep -qw 'fn knowledge_search_returns_matching_entries' crates/roko-serve/src/routes/aggregator.rs && cargo test -p roko-serve routes::aggregator::tests::knowledge_search_returns_matching_entries"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:48Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:13:27Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

`search_knowledge` (`routes/aggregator.rs:584`) returns `{"results": [], "query": q, "timestamp"}` unconditionally.
It is the only knowledge search route a read-scope key can call (`POST /api/neuro/query` needs write scope), so read-only clients and dashboards always see an empty knowledge base.
Fix: back it with the knowledge store query (same scoring as `/api/neuro/query`), bounded and read-only.

Re-checked 2026-09-29: unchanged (stub at crates/roko-serve/src/routes/aggregator.rs:584-593). The existing verify is unsound: no aggregator test calls /knowledge/search, so it passes with the stub in place.

## Notes

- 2026-10-01 (wk-serve2): premise partly wrong. `/api/knowledge/search` is not the only read-scope knowledge search:
  `GET /api/knowledge?q=` and `GET /api/retrieval/query?q=` (`routes/neuro.rs`) already query the store, and every
  GET needs only read scope (`required_scope_for`, `routes/middleware.rs`). The documented `/knowledge/search` route
  was still a stub at BASE `ebdc0f5d5`, so it was implemented.
- 2026-10-01 (wk-serve2): implemented on work/bug-1cb461; cargo verification deferred to the batch check.
  `search_knowledge` ranks entries with `KnowledgeStore::query_hits` (the scoring behind `POST /api/neuro/query`),
  takes `k` (default 10, capped at 100) in the mirage-compatible `{results, query, timestamp}` shape, and adds
  test `knowledge_search_returns_matching_entries`. The verify now guards the cargo filter with a grep.
