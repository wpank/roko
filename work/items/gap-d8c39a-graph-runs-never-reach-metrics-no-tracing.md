+++
id = "gap-d8c39a"
kind = "gap"
title = "Graph runs never reach /metrics: no tracing-to-registry bridge, and serve drops its MetricRegistry"
status = "open"
triage = "verified"
severity = "p2"
size = "M"
goal = "visibility"
subsystem = ["roko-serve", "roko-cli/serve_runtime"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
source = "agent:wk-gates find-4b4344"
discovered_from = "item:find-4b4344"
anchors = ["crates/roko-cli/src/serve_runtime.rs::run_plan_on_local_runtime", "crates/roko-cli/src/graph_task_dispatch/gate_learning.rs::record_gate_verdict_metrics", "crates/roko-core/src/obs/metrics.rs::register_standard_metrics", "crates/roko-agent/src/model_call_service.rs::ModelCallService::with_metrics"]
links = { depends_on = [], blocks = [], related = ["find-4b4344", "find-f489db"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn graph_verify_increments_gate_verdict_metrics' crates/ && cargo test -p roko-cli graph_verify_increments_gate_verdict_metrics"
+++

## Problem

`roko_gate_verdicts_total` stays at zero on `GET /metrics` while `roko serve` runs a plan, however many
verify steps the run settles. The same holds for every other metric that roko emits as a tracing field.

Several paths emit metrics only as tracing fields, in the `tracing-opentelemetry` naming style:
- Graph verify steps: `monotonic_counter.roko_gate_verdicts_total` and
  `histogram.roko_gate_duration_seconds` (`graph_task_dispatch/gate_learning.rs::record_gate_verdict_metrics`,
  added by find-4b4344);
- the test-only `run_gate_once` (`runner/gate_dispatch.rs`);
- `roko_conductor_evaluations_total` (`roko-conductor/src/conductor.rs`);
- `roko_provider_failures_total` (`roko-learn/src/provider_health.rs`);
- `roko_hdc_queries_total` (`roko-neuro/src/knowledge_store/query.rs`).

No subscriber layer turns these fields into samples. There is no `MetricsLayer` and no `tracing_opentelemetry` in
the workspace. `/metrics` renders the `MetricRegistry`, where `register_standard_metrics` registers
`roko_gate_verdicts_total`, so the series appears but is never incremented.

The registry does not reach a Graph run either. `serve_runtime.rs::run_plan_on_local_runtime` takes
`_metrics: Option<Arc<MetricRegistry>>` and drops it.

Expected: a verify step settled by a Graph run that serve hosts increments `roko_gate_verdicts_total` (with
`result` and `rung` labels) and records `roko_gate_duration_seconds` in the registry that `/metrics` renders.

## Why it matters

Goal `visibility`: runs must be observable live in serve. find-4b4344's "Done when" asks for a non-zero
`roko_gate_verdicts_total` on `/metrics` during a served run. That part was blocked on this item.

## Where

- `crates/roko-cli/src/serve_runtime.rs::run_plan_on_local_runtime` receives the registry and ignores it. It calls
  `run_graph_plan`, which builds the `GraphTaskDispatcher`.
- `crates/roko-cli/src/graph_task_dispatch/gate_learning.rs::record_gate_verdict_metrics` is the per-step emission
  point. Both verify loops in `graph_task_dispatch/verification.rs` call it.
- `crates/roko-core/src/obs/metrics.rs`: `MetricRegistry` (`register_counter`, `get_counter`,
  `register_histogram`) and `register_standard_metrics` (:605).
- `crates/roko-agent/src/model_call_service.rs::with_metrics` is the precedent: serve threads its registry into the
  model-call service (`roko-serve/src/service_factory.rs:340`, `:522`).

## Current state

The tracing events are emitted on every Graph verify step since find-4b4344. Nothing counts them.

## Plan

There are two options:
- (A) Thread the registry. Give `GraphTaskDispatcher` (or `GraphFeedbackContext`) an
  `Option<Arc<MetricRegistry>>`, pass serve's registry through `run_plan_on_local_runtime` and `run_graph_plan`,
  and have `record_gate_verdict_metrics` increment the labelled counter and histogram as well as tracing. CLI runs
  have no registry and keep only the tracing events. This is small and explicit, and matches `with_metrics`.
- (B) Install a tracing layer that maps `monotonic_counter.*` and `histogram.*` fields onto a registry. This
  covers every emitter above at once, but it needs a process-wide registry, which the codebase avoids today.

Recommend (A) for gate metrics now, and file (B) separately if the other emitters are wanted.

## Done when

- A served plan run with verify steps shows `roko_gate_verdicts_total{result="pass",...}` above zero on `/metrics`.
- The test `graph_verify_increments_gate_verdict_metrics` checks the registry counter after a verify step.
- The `[[verify]]` command passes.

## Notes

- Keep the tracing fields, which logs and any future layer still read.
- Keep the metric labels low-cardinality (`result`, `rung`). Do not label by plan or task id.
