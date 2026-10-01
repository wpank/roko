+++
id = "bug-a1fc98"
kind = "bug"
title = "WitnessVertex ids depend on serde_json key order, so the same vertex can hash differently in another build"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-agent/safety"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "c7560e213"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-ff95f5"
anchors = ["crates/roko-agent/src/safety/"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-ff95f5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-agent --lib witness_vertex_id_is_canonical"
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
