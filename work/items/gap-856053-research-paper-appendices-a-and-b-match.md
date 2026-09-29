+++
id = "gap-856053"
kind = "gap"
title = "Research paper appendices A and B: match the benchmark and the spec standard as built"
status = "open"
triage = "verified"
severity = "p2"
goal = "whitepaper"
size = "M"
subsystem = ["paper"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e18"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wave-1 reports)"
anchors = ["tmp/cybernetic-harness/paper/sections/A-benchmark.md", "tmp/cybernetic-harness/paper/sections/B-spec-standard.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f tools/paperlint.py && python3 tools/paperlint.py --budget 1.2 --check-identifiers tmp/cybernetic-harness/paper/sections/A-benchmark.md tmp/cybernetic-harness/paper/sections/B-spec-standard.md"
+++

## Problem

Appendices A and B describe ViabilityBench and the spec standard as designed. Both have now begun to exist:
- the benchmark tree, schemas and price snapshot (gap-0580f7, merged in `7bdc61da3`);
- speclint's first slice, SQ01–SQ12 (gap-1cd8d3, merged in `43ac00c28`). Its corpus figure is 75.3% of 551 tasks
  with no acceptance criteria; the prototype's 81.6% came from a word match.

CB.13 is a result claim marked supported.

## Why it matters

The appendices must match the artifacts. Epic spec-f8d196.

## Where

`tmp/cybernetic-harness/paper/sections/A-benchmark.md` and `tmp/cybernetic-harness/paper/sections/B-spec-standard.md`. Sources: `benchmarks/viabilitybench/` (README, schema, speclint)
at HEAD.

## Current state

Both appendices are as designed.

## Plan

1. **Tags:** mark what exists with status tags at a commit, and keep `[[AS-BUILT]]` on the rest.
2. **Numbers:** use speclint's measured figures, with their commit.
3. **CB.13:** retype it to observational, or set it back to pending.
4. **Trim:** keep both appendices within 1.2× budget.

## Done when

- [ ] Both appendices match the merged artifacts, and CB.13 no longer breaks the ledger rule.
- [ ] The `[[verify]]` command passes.

## Notes

- These files are untracked, so edit them in place in the main checkout. Edit only this item's anchored files.
- Keep every `[[RESULT …]]` slot, and use the marker grammar and the `observational` claim type in
  `tmp/cybernetic-harness/paper/00-README.md`.
- The working title is "A Cybernetic Harness for Spec'd Agent Work: Frontier Models Plan, Cheap Models Execute"; the
  thesis is golden path plus cybernetics. Use the status tags in `docs/whitepaper/data/mechanisms.toml`.
