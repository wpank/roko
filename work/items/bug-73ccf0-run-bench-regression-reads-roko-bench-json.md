+++
id = "bug-73ccf0"
kind = "bug"
title = "run_bench_regression reads .roko/bench/*.json, but runs live in runs/, so the regression check never fires"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-serve/routes/bench"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-honestbench's report, checked on work/bug-28becc at abb181f65)"
anchors = ["crates/roko-serve/src/routes/bench.rs", "crates/roko-serve/src/bench.rs"]
lane = "rust-cold"
links = { depends_on = ["bug-28becc"], blocks = [], related = ["bug-28becc"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn bench_regression_reads_the_stored_runs' crates/roko-serve/src/ && cargo test -p roko-serve --lib bench_regression_reads_the_stored_runs"
+++

## Problem

serve's bench regression check, `run_bench_regression` (`crates/roko-serve/src/routes/bench.rs:1141` on bug-28becc's branch), looks for completed runs as `*.json` directly under `.roko/bench` (:1192). Runs are stored in the `runs/` directory, so it finds nothing to compare, and `BenchRegressionReport` (`events.rs:509-511`) is never produced.

## Why it matters

Release: a regression check that can never fire looks like "no regressions".

## Where

`run_bench_regression`, and the run storage layout in `crates/roko-serve/src/bench.rs` (:486).

## Plan

1. Read runs from where the bench writes them (one helper for the runs directory, shared by writer and reader).
2. Add `bench_regression_reads_the_stored_runs`: two stored runs with a regression produce a report.

## Done when

- [ ] A regression between stored runs produces a `BenchRegressionReport`.
- [ ] The `[[verify]]` command passes.
