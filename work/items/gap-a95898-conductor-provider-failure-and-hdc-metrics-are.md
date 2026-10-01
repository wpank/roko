+++
id = "gap-a95898"
kind = "gap"
title = "Conductor, provider-failure and HDC metrics are tracing-only and never reach /metrics"
status = "open"
triage = "unverified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-serve", "roko-cli"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-d8c39a"
anchors = ["crates/roko-cli/src/graph_task_dispatch/gate_learning.rs", "crates/roko-serve/src/lib.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-d8c39a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-serve --lib metrics_include_conductor_and_provider_failures"
+++

## Problem

gap-d8c39a bridged Graph runs' gate metrics to the `/metrics` registry. `roko_conductor_evaluations_total`, `roko_provider_failures_total` and `roko_hdc_queries_total` are still emitted only as tracing, so they never reach the registry (that item's option B).

## Plan

Record them in the shared MetricRegistry where they are emitted. Add a test named `metrics_include_conductor_and_provider_failures`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-01 by wk-streams, working on gap-d8c39a, during the evening close-out round.
