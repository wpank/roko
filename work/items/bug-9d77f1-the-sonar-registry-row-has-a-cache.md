+++
id = "bug-9d77f1"
kind = "bug"
title = "The Sonar registry row has a cache-read price that Perplexity does not publish"
status = "open"
triage = "unverified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-core/pricing"]
created = 2026-10-02
updated = 2026-10-02
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-e2b31a"
anchors = ["crates/roko-core/src/config/model_registry.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-e2b31a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-core --lib sonar_has_no_cache_price"
+++

## Problem

The registry's sonar row sets `cache_read_per_m` 0.0625, and a comment says only Sonar has a cache price, but Perplexity's pricing page lists no cache price for any Sonar model.

## Plan

Drop the cache price (or cite where it comes from), fix the comment, and add a test named `sonar_has_no_cache_price`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-02 by wk-cfg, working on bug-e2b31a, during the overnight close-out round.
