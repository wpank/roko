+++
id = "gap-89f393"
kind = "gap"
title = "Plan-level slice: fixture features that need whole multi-task plans, with hidden whole-feature tests"
status = "open"
triage = "unverified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e12"
discovered_from = "workstreams/assessment/W10-benchmarks-proof.md (rec 12); decided 2026-09-29: evaluation covers single tasks plus a plan-level slice"
anchors = ["benchmarks/viabilitybench/families/plan_slice/"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = ["gap-2790c5"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_plan_slice_reference_passes_and_stub_fails' benchmarks/viabilitybench/tests/test_plan_slice.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/tests/test_plan_slice.py -k test_plan_slice_reference_passes_and_stub_fails -q"
+++

## Problem

The pilot compares single, pre-written tasks. Two things the whitepaper argues are untested:

- **The golden path:** a frontier model plans a multi-task change, cheap models carry it out in parallel, and the
  result integrates.
- **"Faster":** a benchmark unit today is one task.

## Why it matters

Will decided on 2026-09-29 that the evaluation covers single tasks plus a small plan-level slice. This item builds
the slice's fixtures, and a sibling item runs them. It is part of epic spec-567e52.

## Where

The new directory `benchmarks/viabilitybench/families/plan_slice/`. Its location follows D4, which is decided:
ViabilityBench at `benchmarks/viabilitybench/`.

## Current state

Nothing exists yet. W10 rec 12 sketches the slice: 6–10 generated multi-module features whose task DAG is at least 3
wide.

## Plan

1. **Features:** 6–10 of them. Each needs 4–8 tasks across two or more modules, and at least 3 of those tasks can run
   in parallel.
2. **Hidden tests:** each feature gets a whole-feature test suite, written by a different model family and kept out of
   the agent's reach, as for the task families.
3. **Reference and stub:** each feature has a reference implementation that passes and a stub that fails.
4. **What each arm gets:** Roko's arm writes its own plan with the frontier planner; Claude Code gets only the feature
   description.

## Done when

- [ ] 6–10 features exist, each with hidden tests, a reference implementation and a stub.
- [ ] Verifier CI shows that each reference passes and each stub fails.
- [ ] The `[[verify]]` command passes.

## Notes

- Building the fixtures spends nothing; the runs are the sibling item.
- The tests use the benchmark's pinned venv. Will decided on 2026-09-29 that it is created by gap-0580f7.
