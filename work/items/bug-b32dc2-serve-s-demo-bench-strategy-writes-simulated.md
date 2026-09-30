+++
id = "bug-b32dc2"
kind = "bug"
title = "serve's Demo bench strategy writes simulated tokens and cost into runs and the index (pareto, cost-summary)"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-serve/bench"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "0b84bc9fa"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-honestbench's report, checked on work/bug-28becc at abb181f65)"
anchors = ["crates/roko-serve/src/bench.rs"]
lane = "rust-cold"
links = { depends_on = ["bug-28becc"], blocks = [], related = ["bug-28becc", "bug-73ccf0"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn demo_bench_runs_stay_out_of_the_index' crates/roko-serve/src/ && cargo test -p roko-serve --lib demo_bench_runs_stay_out_of_the_index"
+++

## Problem

serve's bench has a `Demo` strategy (`crates/roko-serve/src/bench.rs:163`) that simulates runs. wk-honestbench reports that its simulated tokens and costs are written into the same runs and index as real runs, so the pareto view and the cost summary mix simulated and measured numbers.

## Why it matters

Release: simulated numbers must never be presented as results.

## Where

The `Demo` strategy and the run and index writers in `bench.rs`.

## Plan

1. Mark demo runs as simulated, in every record they write, and keep them out of the index, pareto and cost summaries (or show them only under an explicit demo label).
2. Add `demo_bench_runs_stay_out_of_the_index`.

## Done when

- [ ] No summary or index mixes demo runs with real ones.
- [ ] The `[[verify]]` command passes.

## Notes

- **wk-honestbench (2026-09-30):** Implemented on `work/bug-32d57f` at `2ae0b2887`; cargo verification deferred to the batch check. Demo runs no longer appear in `GET /bench/runs` (index-backed), so the portal's run list will not list them; they stay readable by id with `simulated: true`.
