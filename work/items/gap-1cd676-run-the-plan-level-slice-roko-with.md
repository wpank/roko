+++
id = "gap-1cd676"
kind = "gap"
title = "Run the plan-level slice: Roko with the ladder against Claude Code on whole features"
status = "open"
triage = "unverified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e12"
discovered_from = "workstreams/assessment/W10-benchmarks-proof.md (rec 12); decided 2026-09-29"
anchors = ["benchmarks/viabilitybench/reports/"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = ["gap-89f393", "gap-460230", "gap-60233f", "gap-f30b8e"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "ls -d benchmarks/viabilitybench/reports/plan_slice_* >/dev/null 2>&1 && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/report.py --check benchmarks/viabilitybench/reports/plan_slice_*"
+++

## Problem

Nothing measures whether Roko carries a whole feature from plan to integrated result more cheaply or faster than
Claude Code working alone.

## Why it matters

This is the plan-level slice Will added to the evaluation on 2026-09-29. It gives the whitepaper's §8 its only
plan-level numbers, the first test of "frontier plans, cheap models execute", and the first measurement of "faster".
It is part of epic spec-567e52.

## Where

- The fixtures from gap-89f393.
- The driver and report under `benchmarks/viabilitybench/`.
- Results in `benchmarks/viabilitybench/reports/plan_slice_<date>/`. Under D4, small summaries are committed.

## Current state

Waits for the fixtures, the tier ladder with escalation (gap-460230), the whole-plan gate (gap-60233f) and the
golden-path acceptance test (gap-f30b8e).

## Plan

1. **Two arms:**
   - Roko: the frontier model plans, the ladder executes, and the whole-plan gate checks the result.
   - Claude Code on Opus 5.5, using its own subagents.
2. **Per arm, measure:**
   - verified features;
   - cost per verified feature, including the planner, retries and verification;
   - end-to-end time (makespan).
3. **Seeds and cost:** run one seed first. Cheap-model spend is about $5–10 billed; Claude Code runs on the
   subscription (D42, decided yes).
4. **Results:** commit the summary bundle, with its run ids, under `reports/`.

## Done when

- [ ] A summary bundle is committed under `benchmarks/viabilitybench/reports/plan_slice_<date>/`.
- [ ] Its report shows, for each arm, verified features, cost per verified feature and makespan, with run ids.
- [ ] The `[[verify]]` command passes.

## Notes

- This run is exploratory and not pre-registered as confirmatory. Say so wherever it is cited.
