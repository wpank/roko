+++
id = "bug-aa8893"
kind = "bug"
title = "roko_hdc_queries_total has no production emitter"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-neuro"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "ac3cb2254"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-a95898"
anchors = ["crates/roko-neuro/src/"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-a95898"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rq 'roko_hdc_queries_total' crates/ && cargo test -p roko-neuro --features hdc --lib e24_query_hdc_is_similarity_sorted"
+++

## Problem

`roko_hdc_queries_total` is reached only from `query_hdc`, which only the uncalled `query_by_role_filler` calls, so the metric never moves.

## Plan

Emit it where HDC queries actually run, or drop the metric.

## Done when

- The metric moves in a test, or it is gone.

## Notes

- Reported on 2026-10-02 by wk-streams, working on gap-a95898, during the overnight close-out round.
- 2026-10-02 (wk-streams): implemented on work/gap-b35a57 (dropped); cargo verification deferred to the batch check.
  No production path calls `query_hdc`. HDC does run in production, but only as the similarity term inside every
  keyword query (`score_entry_for_query`, with the `hdc` feature that roko-cli and roko-serve enable), so a
  separate "HDC queries" counter would just count knowledge queries. The counter field is gone, and `query_hdc`
  still logs its result count and top similarity.
