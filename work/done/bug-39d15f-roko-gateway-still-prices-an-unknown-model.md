+++
id = "bug-39d15f"
kind = "bug"
title = "roko-gateway still prices an unknown model at Sonnet rates"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-gateway"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "39feebc07"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-ad0d39"
anchors = ["crates/roko-gateway/src/"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-ad0d39"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rn 'sonnet_fallback' crates/roko-gateway/src --include=*.rs"

[closed]
at = 2026-10-02
at_ts = "2026-10-01T23:45:17Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T18:53:00Z"
forced = false
evidence = "Gate 6d on 9eacfde5f plus its fixes, re-checked at c9e78d12d and merged as 39feebc07 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean; lib tests pass (roko-cli 3407, roko-agent 2276, roko-core 1980, roko-learn 1228, roko-serve 1012, roko-acp 219, roko-compose 562, roko-execution 192, roko-gateway 43); all eight canaries with the new default-isolation canary, golden_path_suite, secret_canary, C2, graph_plan_callers, cost_dedup, phase0_wiring, run_serve_share, graph_timeout_matrix (6), plan_prepare_full and the gateway pipeline contract pass; bin tests pass; scripts/test_run_evidence_graph.py 9/9 against the gate binary; Cargo.lock unchanged; route snapshot matches. Implemented in this round; the item's notes name the change and its test."
+++

## Problem

gap-ad0d39 made roko-learn leave unknown models unpriced. The gateway still prices them with `sonnet_fallback`, so gateway cost events disagree with the cost log.

## Plan

Leave unknown models unpriced in the gateway too (cost unknown, warn once), as roko-learn does.

## Done when

- The verify passes, and the gateway's cost tests are updated.

## Notes

- Reported on 2026-10-01 by wk-model-truth, working on gap-ad0d39, during the evening close-out round.
- 2026-10-01 (wk-model-truth): implemented on work/bug-3aa61f; cargo verification deferred to the batch check.
  `CostTracker::compute_cost` returns an all-zero `CostResult` for a model the table does not price and logs it
  once through roko-learn's `warn_unpriced_model` (now `pub`, so the two log a model once between them);
  `sonnet_fallback` is gone. Test: `cost_track_leaves_an_unknown_model_unpriced` (was
  `cost_track_unknown_model_uses_sonnet_fallback`); docs/v3's gateway cost pages say so. Consequence: the
  gateway's budget preflight can no longer price, so no longer blocks, a call to an unpriced model.
