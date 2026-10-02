+++
id = "bug-1f81ab"
kind = "bug"
title = "builtin_pricing's prefix match prices smaller models at their larger sibling's rate"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-core/pricing"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "39feebc07"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-0c0747"
anchors = ["crates/roko-core/src/config/model_registry.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-0c0747"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-core --lib builtin_pricing_prefix"

[closed]
at = 2026-10-02
at_ts = "2026-10-01T23:45:15Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T18:53:00Z"
forced = false
evidence = "Gate 6d on 9eacfde5f plus its fixes, re-checked at c9e78d12d and merged as 39feebc07 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean; lib tests pass (roko-cli 3407, roko-agent 2276, roko-core 1980, roko-learn 1228, roko-serve 1012, roko-acp 219, roko-compose 562, roko-execution 192, roko-gateway 43); all eight canaries with the new default-isolation canary, golden_path_suite, secret_canary, C2, graph_plan_callers, cost_dedup, phase0_wiring, run_serve_share, graph_timeout_matrix (6), plan_prepare_full and the gateway pipeline contract pass; bin tests pass; scripts/test_run_evidence_graph.py 9/9 against the gate binary; Cargo.lock unchanged; route snapshot matches. Implemented in this round; the item's notes name the change and its test."
+++

## Problem

`builtin_pricing` falls back to a prefix match, so o3-mini is priced as o3, gpt-4o-mini as gpt-4o, gemini-2.5-flash-lite as 2.5 Flash, and sonar-reasoning(-pro) as sonar. costs_db's exact lookup disagrees for these models.

## Plan

Match the longest known slug, and refuse a prefix that crosses a model-family boundary (e.g. `-mini`, `-lite`). Add the missing rows, from dated price pages, or leave them unpriced. Add a test named `builtin_pricing_prefix_*`.

## Done when

- `cargo test -p roko-core --lib builtin_pricing_prefix` passes.

## Notes

- Reported on 2026-10-01 by wk-model-truth, working on bug-0c0747, during the evening close-out round.
- 2026-10-01 (wk-model-truth): implemented on work/bug-3aa61f; cargo verification deferred to the batch check.
  New `model_registry::is_snapshot_of`: a slug takes a key's rates only when its suffix is a date or zero-padded
  version (`-20250514`, `-2024-08-06`, `-001`), `-latest` or `-preview`; `builtin_pricing` and roko-learn's
  `CostTable::lookup` both use it. Added rows checked against the price pages on 2026-10-01: o3-mini ($1.10/$4.40,
  $0.55 cached), gpt-4o-mini ($0.15/$0.60, $0.075), gemini-2.5-flash-lite ($0.10/$0.40, $0.01), sonar-reasoning-pro
  ($2/$8, no cache price); costs_db now takes those from the registry and drops sonar-reasoning, which Perplexity's
  page no longer lists. Left unpriced, with no registry row: gpt-5.4-nano, o3-pro, sonar-reasoning,
  sonar-deep-research (costs_db keeps its own estimate), glm-5.2 and other non-snapshot suffixes. Tests:
  `builtin_pricing_prefix_takes_a_snapshot_suffix`, `builtin_pricing_prefix_refuses_another_models_suffix`.
