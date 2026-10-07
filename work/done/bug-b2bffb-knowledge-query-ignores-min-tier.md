+++
id = "bug-b2bffb"
kind = "bug"
title = "Knowledge query ignores min_tier"
status = "done"
triage = "verified"
severity = "p2"
subsystem = ["roko-serve/neuro"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/routes/neuro.rs:35"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = 'test "$(grep -c min_tier crates/roko-serve/src/routes/neuro.rs)" -gt 1'

[[verify]]
command = "test \"$(grep -c min_tier crates/roko-serve/src/routes/neuro.rs)\" -gt 1 && grep -qw 'fn query_filters_by_min_tier' crates/roko-serve/src/routes/neuro.rs && cargo test -p roko-serve routes::neuro::tests::query_filters_by_min_tier"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:25Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:13:27Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

The knowledge query request declares `min_tier: Option<String>` (`routes/neuro.rs:35`), but it is the field's only reference in the file: it is never read.
Callers asking for working or consolidated knowledge also get transient entries.
Fix: parse `min_tier`, filter by tier (400 on unknown tier names) and add a route test.

## Notes

- 2026-10-01 (wk-serve2): implemented on work/bug-1cb461; cargo verification deferred to the batch check.
  `POST /api/neuro/query` validates `min_tier` (case-insensitive `transient|working|consolidated|persistent`, 400
  otherwise) and keeps only entries at or above it, ranking every match before filtering so lower tiers cannot
  crowd out `limit`. Test `query_filters_by_min_tier` covers no filter, `Working`, `limit` with a filter, and an
  unknown tier. The verify now guards the cargo filter with a grep.
