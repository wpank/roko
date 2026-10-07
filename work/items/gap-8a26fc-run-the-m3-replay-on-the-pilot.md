+++
id = "gap-8a26fc"
kind = "gap"
title = "Run the M3 replay on the pilot matrix and file it as a labelled smoke test, task 6125"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
hold = "needs the pilot records: Pilot A (gap-c33709) and Pilot B (gap-327242) haven't run"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "tmp/backlog/2026-10-02-complete-and-wire 6125 (blocked in wave 8, PK49)"
discovered_from = "gap-7ec3ef"
anchors = ["benchmarks/viabilitybench/analysis"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["gap-7ec3ef", "gap-c33709", "gap-327242"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -n \"$(find benchmarks/viabilitybench/reports -name econ-report.json 2>/dev/null)\""
+++

## Problem

Task 6125 of PK49 (gap-7ec3ef, gated in wave 8) runs the M3 replay on the pilot matrix and files the result as a
labelled smoke test: an `econ-report.json` under `benchmarks/viabilitybench/reports/` that S10's replay view renders
(reliability diagram, Pareto set, pass^k). No pilot has run (gap-c33709 tracks Pilot A, gap-327242 Pilot B), so
there is no matrix to replay. PK49's worker wrote no report rather than fabricate one.

## Why it matters

It is S04's first-slice evidence that the M3 replay works end to end on real records, and the first data S10 shows.

## Where

`benchmarks/viabilitybench/analysis/econ.py` (6123, merged), `roko learn self-model replay` (6124, merged), and the
pilot records under `$VB_RESULTS`.

## Current state

The replay and the economics report run on fixtures; the pilots haven't run.

## Plan

1. After Pilots A and B have run, build the run-record matrix from their records and run the replay as 6125 says.
2. Commit the report, labelled a smoke test (not H-evidence).

## Done when

- [ ] The `[[verify]]` command passes.

## Notes

- Full spec: `tmp/backlog/2026-10-02-complete-and-wire/6125-run-the-m3-replay-on-the-pilot-matrix-and-file-it-as-a.md`.
- Left PK49's package item at gate 8c (2026-10-03).
