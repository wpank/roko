+++
id = "gap-4adfa7"
kind = "gap"
title = "PatternMiner's trigram patterns have no production reader"
status = "done"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-learn"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "39feebc07"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-d0f52f"
anchors = ["crates/roko-learn/src/pattern_discovery.rs", "crates/roko-learn/src/runtime_feedback/mod.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-d0f52f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rnE 'pattern_miner|pattern_discovery_every_n|patterns_ingested' crates --include=*.rs && cargo test -p roko-learn --lib update_frequency_separation"

[closed]
at = 2026-10-02
at_ts = "2026-10-01T23:45:20Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T18:51:09Z"
forced = false
evidence = "Gate 6d on 9eacfde5f plus its fixes, re-checked at c9e78d12d and merged as 39feebc07 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean; lib tests pass (roko-cli 3407, roko-agent 2276, roko-core 1980, roko-learn 1228, roko-serve 1012, roko-acp 219, roko-compose 562, roko-execution 192, roko-gateway 43); all eight canaries with the new default-isolation canary, golden_path_suite, secret_canary, C2, graph_plan_callers, cost_dedup, phase0_wiring, run_serve_share, graph_timeout_matrix (6), plan_prepare_full and the gateway pipeline contract pass; bin tests pass; scripts/test_run_evidence_graph.py 9/9 against the gate binary; Cargo.lock unchanged; route snapshot matches. Implemented in this round; the item's notes name the change and its test."
+++

## Problem

`LearningRuntime::pattern_miner()` is only fed episodes. Nothing in production reads its trigram patterns, so the loop is dead.

## Plan

Wire the patterns into something that uses them (prompt hints or routing), with a test, or delete the miner.

## Done when

- `LearningRuntime`'s miner is gone: no `pattern_miner`, `pattern_discovery_every_n` or `patterns_ingested` is left
  under `crates/`, and `cargo test -p roko-learn --lib update_frequency_separation` passes.

## Notes

- Reported on 2026-10-01 by wk-learn2, working on gap-d0f52f, during the evening close-out round.
- 2026-10-01 (wk-learn2): implemented on work/gap-14f08e; cargo verification deferred to the batch check.
  Deleted rather than wired. The runtime's miner was in-memory only, never saved, and fed only every 20th episode's
  gate names, so it held nothing a reader could use. Its field, `pattern_miner()`, the `pattern_discovery_every_n`
  cadence and `LearningUpdate::patterns_ingested` are gone. The `PatternMiner` type stays: roko-neuro's tier
  progression mines episodes with its own miner on demand. The verify now checks for the deletion.
