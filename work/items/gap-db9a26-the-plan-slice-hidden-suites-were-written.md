+++
id = "gap-db9a26"
kind = "gap"
title = "The plan-slice hidden suites were written by the arms' own model family and need a cross-family review"
status = "open"
triage = "unverified"
severity = "p1"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/families/plan_slice"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:47, wk-bench-slice's report on gap-89f393)"
anchors = ["benchmarks/viabilitybench/families/plan_slice/features/", "benchmarks/viabilitybench/families/plan_slice/README.md"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-89f393", "gap-1cd676"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rq 'cross_family_review = \"pending\"' benchmarks/viabilitybench/families/plan_slice/features/"
+++

## Problem

The hidden whole-feature suites of the six plan-slice features (`families/plan_slice/features/pl01…pl06`) were written by `claude-opus-5-5`: the model family of the `fd_claude` arm and of `roko_plan`'s planner. S09 §4.9 and D13 require hidden suites from a different model family, so the suite's author can't share blind spots with the arms it judges. Every `feature.toml` records `cross_family_review = "pending"` (README, "Known gaps").

## Why it matters

A validity threat for the pilot benchmark (epic spec-567e52): a suite from the same family may miss exactly the mistakes both arms tend to make, which inflates VF for both and hides the differences between them. The suites are also static files, so only isolation and canaries keep them from agents.

## Where

The anchors. Each `feature.toml` holds `author` and `cross_family_review`.

## Current state

At BASE all six features are `pending`, and no review has run.

## Plan

1. Pick a reviewer model from another family (not Anthropic), pinned by id.
2. For each feature, give it the feature description and the suite. Ask it for requirements the suite misses or checks wrongly, and for extra cases; or have it rewrite the suite from the description alone.
3. Merge the accepted changes and re-run `slicekit.py selftest` (the reference passes, the stub fails).
4. Record `cross_family_review = "<model id>, <date>, <outcome>"` in each `feature.toml`.

## Done when

- [ ] Every feature records a completed cross-family review, and `selftest` passes.
- [ ] The `[[verify]]` command passes.

## Notes

- This costs model calls on a non-Anthropic provider. Confirm the budget line and the provider before running.
- Keep the reviewer's prompts and outputs with the fixtures' provenance, outside the tree an agent sees.
