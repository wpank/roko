+++
id = "bug-b2bffb"
kind = "bug"
title = "Knowledge query ignores min_tier"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-serve/neuro"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/routes/neuro.rs:35"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = 'test "$(grep -c min_tier crates/roko-serve/src/routes/neuro.rs)" -gt 1'

[[verify]]
command = "test \"$(grep -c min_tier crates/roko-serve/src/routes/neuro.rs)\" -gt 1 && grep -qw 'fn query_filters_by_min_tier' crates/roko-serve/src/routes/neuro.rs && cargo test -p roko-serve routes::neuro::tests::query_filters_by_min_tier"
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
