+++
id = "bug-a1fc98"
kind = "bug"
title = "WitnessVertex ids depend on serde_json key order, so the same vertex can hash differently in another build"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-agent/safety"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "4dc345a29"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-ff95f5"
anchors = ["crates/roko-agent/src/safety/"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-ff95f5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-agent --lib witness_vertex_id_is_canonical"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T00:39:00Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T22:34:21Z"
forced = false
evidence = "Gate 6e on ddf47dbd6 plus its fixes, re-run at 3ac297a00 and merged as 4dc345a29 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 10 crates; lib tests pass (roko-cli 3420, roko-agent 2289, roko-core 1984, roko-learn 1230, roko-serve 1013, roko-graph 488, roko-conductor 316, roko-acp 220, roko-execution 193, roko-dreams 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp integration, smoke, graph_plan_callers, graph_timeout_matrix and plan_conversion pass; bin 445; scripts/test_run_evidence_graph.py 9/9 against the gate binary; Cargo.lock unchanged. Implemented in this round; the item's notes name the change and its test."
+++

## Problem

A WitnessVertex id hashes `serde_json::to_vec(content)`. Key order flips with serde_json's `preserve_order` feature, which workspace builds unify (see the memory on serde_json preserve_order), so the same vertex can get a different id in another build.

## Plan

Hash a canonical encoding (sorted keys). Add a test named `witness_vertex_id_is_canonical` that checks two key orders give one id.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-01 by wk-tamper, working on gap-ff95f5, during the evening close-out round.
- 2026-10-02 (wk-guard2): implemented on work/bug-7f15df; cargo verification deferred to the batch check.
- `WitnessVertex::new` and `verify_id` now hash the content's RFC 8785 canonical JSON (sorted keys, no whitespace), using the existing `roko_core::config::fingerprint::canonical_json`, the helper gap-ff95f5's provenance digests also use. Test `witness_vertex_id_is_canonical` builds the same content with keys inserted out of order, and checks its id equals the in-order vertex's and the BLAKE3 of the canonical text.
- Old logs: `WitnessLogger` had no production writer before gap-ff95f5, whose vertices come from `WitnessVertex::new` and so get canonical ids. A vertex written by an older build verifies only if its content was already in canonical form (sorted keys; integers, strings and nesting are encoded the same way).
