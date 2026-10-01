+++
id = "bug-ddd5bd"
kind = "bug"
title = "error_pattern_store's append_preserves_first_seen_timestamp fails when two appends share a timestamp tick"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-learn/error_pattern_store"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "814fe5e90"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (the coordinator's report)"
anchors = ["crates/roko-learn/src/error_pattern_store.rs"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = ["bug-779ae7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'fn append_at' crates/roko-learn/src/error_pattern_store.rs && cargo test -p roko-learn --lib append_preserves_first_seen_timestamp"
+++

## Problem

`append_preserves_first_seen_timestamp` (`crates/roko-learn/src/error_pattern_store.rs:903`) appends twice, then asserts that `first_seen` is unchanged and that `last_seen` differs from it (`assert_ne!`, :914). Both appends take the wall-clock time. Under load, or on a coarse clock, they can land in the same tick, and the test fails. It was seen in batch 19 under load, and passes alone.

## Why it matters

Hygiene (epic spec-9a3131): another load flake (bug-779ae7). p3.

## Where

The store's append path and the test.

## Plan

1. Add an `append_at(…, now)` variant (the plain `append` calls it with the current time), and have the test pass two explicit, distinct times.

## Done when

- [ ] The test doesn't depend on the clock.
- [ ] The `[[verify]]` command passes.

## Notes

- **wk-honestbench (2026-10-01):** Implemented on `work/bug-ddd5bd` at `8424c410e`; cargo verification deferred to the batch check. `append` and `observe_gate_failure` keep their signatures and pass `Utc::now()` to the new `append_at` / `observe_gate_failure_at`.
