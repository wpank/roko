+++
id = "gap-a95898"
kind = "gap"
title = "Conductor, provider-failure and HDC metrics are tracing-only and never reach /metrics"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-serve", "roko-cli"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "4dc345a29"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-d8c39a"
anchors = ["crates/roko-cli/src/graph_task_dispatch/gate_learning.rs", "crates/roko-serve/src/lib.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-d8c39a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn metrics_include_conductor_and_provider_failures' crates/roko-serve/src/ && grep -rqw 'fn conductor_counts_evaluations_in_attached_registry' crates/roko-conductor/src/ && cargo test -p roko-serve --lib metrics_include_conductor_and_provider_failures && cargo test -p roko-conductor --lib conductor_counts_evaluations_in_attached_registry"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T00:39:03Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T22:34:25Z"
forced = false
evidence = "Gate 6e on ddf47dbd6 plus its fixes, re-run at 3ac297a00 and merged as 4dc345a29 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 10 crates; lib tests pass (roko-cli 3420, roko-agent 2289, roko-core 1984, roko-learn 1230, roko-serve 1013, roko-graph 488, roko-conductor 316, roko-acp 220, roko-execution 193, roko-dreams 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp integration, smoke, graph_plan_callers, graph_timeout_matrix and plan_conversion pass; bin 445; scripts/test_run_evidence_graph.py 9/9 against the gate binary; Cargo.lock unchanged. Implemented in this round; the item's notes name the change and its test."
+++

## Problem

gap-d8c39a bridged Graph runs' gate metrics to the `/metrics` registry. `roko_conductor_evaluations_total`, `roko_provider_failures_total` and `roko_hdc_queries_total` are still emitted only as tracing, so they never reach the registry (that item's option B).

## Plan

Record them in the shared MetricRegistry where they are emitted. Add a test named `metrics_include_conductor_and_provider_failures`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-01 by wk-streams, working on gap-d8c39a, during the evening close-out round.
- 2026-10-02 (wk-streams): implemented on work/gap-b35a57 for the conductor and provider failures; cargo
  verification deferred to the batch check.
  - `ProviderHealthRegistry::attach_metrics` and `Conductor::attach_metrics` take the registry through a shared
    reference (both live in an `Arc` before serve's registry is in reach) and count beside the tracing fields:
    `roko_provider_failures_total{provider, error_type}` (the `ErrorClass` name) in `record_failure` and
    `record_exhaustion`, and `roko_conductor_evaluations_total` in `evaluate_full`. Names, help and labels are
    descriptors in `roko_core::obs::schema`, kept out of `CANONICAL_METRICS`, which the sidecars share and a test
    counts.
  - Serve attaches its registry to its own provider health registry (`state.rs`) and lists both families on
    `/metrics` from startup. A run serve hosts attaches `GraphPlanRunParams::metrics` (gap-d8c39a) to the run's
    provider health registry and conductor (`plan_runner.rs`).
  - Tests: `metrics_include_conductor_and_provider_failures` (roko-serve, through serve's own wiring) and
    `conductor_counts_evaluations_in_attached_registry` (roko-conductor). roko-serve does not depend on
    roko-conductor, and adding that dependency would need a lockfile update, so the conductor's counting is tested
    in its own crate; the verify runs both.
  - `roko_hdc_queries_total` is not wired: `KnowledgeStore::query_hdc` runs only from `query_by_role_filler`, which
    nothing calls, so no production path emits it. Worth its own item if HDC lookup gets a caller.
