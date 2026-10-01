+++
id = "bug-c1f6b8"
kind = "bug"
title = "roko serve's model calls write no costs.jsonl rows, so cost totals miss all serve spend"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-serve"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "39feebc07"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-aad63e"
anchors = ["crates/roko-serve/src/dispatch.rs", "crates/roko-agent/src/model_call_service.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-aad63e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-serve --lib serve_calls_write_cost_rows"

[closed]
at = 2026-10-02
at_ts = "2026-10-01T23:45:18Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T18:51:09Z"
forced = false
evidence = "Gate 6d on 9eacfde5f plus its fixes, re-checked at c9e78d12d and merged as 39feebc07 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean; lib tests pass (roko-cli 3407, roko-agent 2276, roko-core 1980, roko-learn 1228, roko-serve 1012, roko-acp 219, roko-compose 562, roko-execution 192, roko-gateway 43); all eight canaries with the new default-isolation canary, golden_path_suite, secret_canary, C2, graph_plan_callers, cost_dedup, phase0_wiring, run_serve_share, graph_timeout_matrix (6), plan_prepare_full and the gateway pipeline contract pass; bin tests pass; scripts/test_run_evidence_graph.py 9/9 against the gate binary; Cargo.lock unchanged; route snapshot matches. Implemented in this round; the item's notes name the change and its test."
+++

## Problem

roko serve's model calls (template dispatch, distillation) write feedback rows and gateway events but no `costs.jsonl` rows. `roko show costs` and the budget totals therefore miss all serve spend.

## Plan

Have the shared ModelCallService's feedback sink append the cost row, once, as the CLI does. Add a test named `serve_calls_write_cost_rows`.

## Done when

- `cargo test -p roko-serve --lib serve_calls_write_cost_rows` passes.

## Notes

- Reported on 2026-10-01 by wk-learn2, working on bug-aad63e, during the evening close-out round.
- 2026-10-01 (wk-learn2): implemented on work/gap-14f08e; cargo verification deferred to the batch check.
  `FeedbackService::with_cost_records` appends one `costs.jsonl` row per model call that reached a provider; serve's
  shared feedback service and template dispatch's `ModelCallFeedbackRecorder` opt in. `FeedbackEvent::ModelCall`
  gains `cache_hit`, set by `ModelCallService` on a cached answer, which gets no row; a call that reported no usage
  gets none either (its cost is unknown). Tests: `serve_calls_write_cost_rows`,
  `cost_records_skip_cached_answers_and_calls_without_usage`, and a cost-row check in
  `template_dispatch_records_feedback_and_provider_health`.
