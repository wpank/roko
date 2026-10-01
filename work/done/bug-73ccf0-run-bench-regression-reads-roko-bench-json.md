+++
id = "bug-73ccf0"
kind = "bug"
title = "run_bench_regression reads .roko/bench/*.json, but runs live in runs/, so the regression check never fires"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-serve/routes/bench"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "2ae9d2a7f"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-honestbench's report, checked on work/bug-28becc at abb181f65)"
anchors = ["crates/roko-serve/src/routes/bench.rs", "crates/roko-serve/src/bench.rs"]
lane = "rust-cold"
links = { depends_on = ["bug-28becc"], blocks = [], related = ["bug-28becc"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn bench_regression_reads_the_stored_runs' crates/roko-serve/src/ && cargo test -p roko-serve --lib bench_regression_reads_the_stored_runs"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 89028c96f. The regression check reads stored runs through bench::load_bench_runs. Batch 14 gate: first run on 4e030ab47 (check, clippy clean; tests pass: roko-cli 3171, roko-agent 2263, roko-core 1952, roko-learn 1203, roko-serve 986, roko-graph 472, roko-fs 259, roko-neuro 239), then re-gated on 8ce3bb131 (same code as MAIN 2ae9d2a7f) after the coordinator's rustfmt commits and serve-sec's bug-633b68 root fix: check, nightly fmt, clippy -p roko-cli -p roko-serve -p roko-core -p roko-agent -p roko-learn --keep-going -D warnings clean; roko-cli lib 3172 passed (one sibling-settle race flake passes alone, bug-779ae7); --test secret_canary 11 passed; --test secrets_and_git_guard_canary 1 passed, 1 ignored (bug-0d9ac4). Verify: its test passes in that run and its static checks pass on MAIN."
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

## Notes

- **wk-honestbench (2026-09-30):** Implemented on `work/bug-32d57f` at `61fa24884`; cargo verification deferred to the batch check. `bench::load_bench_runs` is the shared reader for the runs directory that `save_bench_run` writes.
