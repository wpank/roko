+++
id = "gap-942f88"
kind = "gap"
title = "Measure the spec refiner's cost on a live gpt-oss-120b run (within $0.02), task 3236"
status = "open"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
hold = "needs Will's spend approval for one live gpt-oss-120b run"
subsystem = ["benchmarks/viabilitybench/specops"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "tmp/backlog/2026-10-02-complete-and-wire 3236 (blocked in wave 6, PK24)"
anchors = ["benchmarks/viabilitybench/specops"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'within the $0.02 cap: yes' benchmarks/viabilitybench/reports/refiner-cost/summary.md"
+++

## Problem

Task 3236 of PK24 (gap-e120a1, merged in 730b43d91) measures what the spec refiner (3235) costs on a real cheap model:
one ledgered run on gpt-oss-120b, with a committed `benchmarks/viabilitybench/reports/refiner-cost/summary.md` that
states whether the refiner stayed within its $0.02 cap. Wave 6 made no live calls, so the report doesn't exist.

## Why it matters

The refiner runs before every cheap execution in the S07 design; its cost per task has to be small and measured.

## Where

`benchmarks/viabilitybench/specops/refine.py` (3235), the cost report under `reports/refiner-cost/`.

## Current state

The refiner, critic and ambiguity probe exist and are tested against a stub model; nothing has run live.

## Plan

1. With Will's spend approval, run the refiner live on gpt-oss-120b as task 3236 says, through the metered proxy so
   the spend is ledgered.
2. Commit the summary with the measured cost and the cap check.

## Done when

- [ ] The `[[verify]]` command passes.

## Notes

- Full spec: `tmp/backlog/2026-10-02-complete-and-wire/3236-*.md`.
