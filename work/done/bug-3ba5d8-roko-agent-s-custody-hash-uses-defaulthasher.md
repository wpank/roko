+++
id = "bug-3ba5d8"
kind = "bug"
title = "roko-agent's custody hash uses DefaultHasher, unstable across Rust releases and unlike roko-cli's SHA-256 chain"
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
command = "cargo test -p roko-agent --lib custody_hash_is_sha256"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T00:38:59Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T22:34:21Z"
forced = false
evidence = "Gate 6e on ddf47dbd6 plus its fixes, re-run at 3ac297a00 and merged as 4dc345a29 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 10 crates; lib tests pass (roko-cli 3420, roko-agent 2289, roko-core 1984, roko-learn 1230, roko-serve 1013, roko-graph 488, roko-conductor 316, roko-acp 220, roko-execution 193, roko-dreams 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp integration, smoke, graph_plan_callers, graph_timeout_matrix and plan_conversion pass; bin 445; scripts/test_run_evidence_graph.py 9/9 against the gate binary; Cargo.lock unchanged. Implemented in this round; the item's notes name the change and its test."
+++

## Problem

`Custody::compute_hash` in roko-agent says SHA-256 but uses `DefaultHasher`, which is not stable across Rust releases. So `CustodyLogger::log_chained` and `verify_chain` disagree with roko-cli's SHA-256 chain (gap-ff95f5), and a toolchain upgrade can break the verification of old chains.

## Plan

Hash with SHA-256 as documented. Add a test named `custody_hash_is_sha256` that pins a known digest. Old chains need a migration note: verify them with the old hasher once, or mark them legacy.

## Done when

- The test passes, and the migration is noted.

## Notes

- Reported on 2026-10-01 by wk-tamper, working on gap-ff95f5, during the evening close-out round.
- 2026-10-02 (wk-guard2): implemented on work/bug-7f15df; cargo verification deferred to the batch check.
- `Custody::compute_hash` is now the lowercase hex SHA-256 of `prev_hash` (empty for the first record) followed by the record's JSON without `prev_hash` and `hash`. That is roko-cli's custody chain formula (`crates/roko-cli/src/custody.rs::compute_hash`), so `seal`, `verify_hash`, `CustodyLogger::log_chained` and `verify_chain` now agree with the chains `roko knowledge custody` writes, and the link to `prev_hash` is part of the digest. roko-agent gains the workspace `sha2` dependency (Cargo.lock: roko-agent now lists `sha2 0.10.9`). Test `custody_hash_is_sha256` pins two chained digests computed independently, and checks that a chain logged with `log_chained` verifies.
- Migration: no old chains need converting. roko-agent's `seal`, `log_chained` and `verify_chain` had no callers outside their own module, so no chain on disk carries the old 16-hex-digit `DefaultHasher` digest, and the chains roko-cli wrote already use the SHA-256 form. A record sealed by an old build would fail `verify_chain` at that record (fail closed); it can't be re-verified reliably anyway, because `DefaultHasher` output changes between Rust releases. Follow-up: once gap-ff95f5 lands, roko-cli's `canonical_payload` and `compute_hash` can call `Custody::compute_hash`, leaving one copy of the formula.
