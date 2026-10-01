+++
id = "bug-ddd5bd"
kind = "bug"
title = "error_pattern_store's append_preserves_first_seen_timestamp fails when two appends share a timestamp tick"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-learn/error_pattern_store"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "1bf49188d"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (the coordinator's report)"
anchors = ["crates/roko-learn/src/error_pattern_store.rs"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = ["bug-779ae7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'fn append_at' crates/roko-learn/src/error_pattern_store.rs && cargo test -p roko-learn --lib append_preserves_first_seen_timestamp"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T09:00:18Z"
by = "coordinator (session 7622b882)"
forced = false
evidence = "Batch 20c gate on fcdaf32ae/ca5645373 (MAIN 1bf49188d has the same crates and portal): check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-acp/agent/cli/core/dreams/gate/learn/neuro/serve; lib tests roko-cli 3273, roko-agent 2278, roko-core 1962, roko-learn 1209, roko-serve 989, roko-gate 692, roko-neuro 239, roko-acp 199, roko-dreams 100 all pass; extras: C1 1/1, C7 2/2, learn_paths 7, cost_comparison 1, bin 429, verify loop 10/10, speclint 91, including append_preserves_first_seen_timestamp at two explicit times. Merged bc2957ba1 (work/bug-ddd5bd 33a131c4e)."
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
