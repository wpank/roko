+++
id = "bug-e2b31a"
kind = "bug"
title = "costs_db's Perplexity per-request fees mix search-context tiers"
status = "open"
triage = "unverified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-learn/cost"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-1f81ab"
anchors = ["crates/roko-learn/src/costs_db.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-1f81ab"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-learn --lib perplexity_request_fees"
+++

## Problem

costs_db's Perplexity per-request fees mix tiers. sonar uses the low fee ($5 per 1K requests) and sonar-pro the high fee ($14 per 1K), while sonar-reasoning-pro's $8 per 1K matches no tier on the price page ($6, $10 or $14).

## Plan

Pick one search-context tier, record which, and price every model at it from the dated page. Add a test named `perplexity_request_fees_*`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-01 by wk-model-truth, working on bug-1f81ab, during the evening close-out round.
