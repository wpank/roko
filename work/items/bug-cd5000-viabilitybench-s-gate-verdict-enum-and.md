+++
id = "bug-cd5000"
kind = "bug"
title = "ViabilityBench's gate_verdict enum and metrics.py's NOT_PASSED don't list already_satisfied"
status = "done"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/schema", "benchmarks/viabilitybench/analysis"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "802117c17"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-tamper's report)"
anchors = ["benchmarks/viabilitybench/schema/run-record.schema.json", "benchmarks/viabilitybench/analysis/metrics.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-9eb1e1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'already_satisfied' benchmarks/viabilitybench/schema/run-record.schema.json && grep -q 'already_satisfied' benchmarks/viabilitybench/analysis/metrics.py"

[closed]
at = 2026-10-01
by = "coordinator (session 7622b882)"
evidence = "Merged 802117c17 (work/tamper-l8 at 540d20f41): run-record schema enum gains already_satisfied; run_roko takes each attempt verdict from the episode extra.outcome and records an already_satisfied plan as failed/already_satisfied instead of infra_error; metrics count already_satisfied_runs as their own bucket, never a pass, false green or VS (S05 0.1(a), App. D.1, S01 4.3). ViabilityBench suite on MAIN after the merge: 373 passed, 2 skipped."
+++

## Problem

gap-9eb1e1 added a task outcome, `already_satisfied`: a `--fresh` rerun whose correct output is already in the tree. The benchmark's run-record schema allows `gate_verdict` only as `passed`, `unverified`, `forced_accept` or null (`schema/run-record.schema.json:90`), and `analysis/metrics.py`'s `NOT_PASSED` is `("unverified", "forced_accept")` (:76). A Roko attempt that settles as `already_satisfied` fails validation, or would be miscounted.

## Why it matters

Pilot benchmark (epic spec-567e52): the Roko arm's records must accept every verdict Roko can produce. p3.

## Plan

1. Add `already_satisfied` to the schema's enum. Decide whether it counts as a reported pass (probably not: it isn't the arm's work) and add it to `NOT_PASSED` accordingly.
2. Map it in `run_roko.py` if needed, and add a test.

## Done when

- [ ] Both files know `already_satisfied`.
- [ ] The `[[verify]]` command passes.
