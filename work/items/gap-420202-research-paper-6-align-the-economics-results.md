+++
id = "gap-420202"
kind = "gap"
title = "Research paper §6: align the economics results template with the thesis, add the plan-level template, and trim"
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
anchors = ["tmp/cybernetic-harness/paper/sections/06-results-economics.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f tools/paperlint.py && python3 tools/paperlint.py --budget 1.2 --check-identifiers tmp/cybernetic-harness/paper/sections/06-results-economics.md"

[[verify]]
command = "grep -q 'RESULT PL' tmp/cybernetic-harness/paper/sections/06-results-economics.md"
+++

## Problem

§6 (about 5.3k words, 2.2× its budget) is a results template written before the golden-path thesis and before the
plan-level slice was decided (2026-09-29). It has no template for the slice.

## Why it matters

This is where the headline "cheaper at equal quality" result lands. Epic spec-f8d196.

## Where

`tmp/cybernetic-harness/paper/sections/06-results-economics.md`. Sources: the decided scope in §5 (gap-ac4ce8), S09's arms, and the E12 items.

## Current state

Every `[[RESULT H1/H2 …]]` slot exists. None covers the plan-level slice.

## Plan

1. **Framing:** set §6 in the golden-path framing (single tasks, plus the exploratory plan-level slice).
2. **Plan-level slots:** add `[[RESULT PL: …]]` slots, labelled exploratory: makespan and cost per verified feature.
3. **Trim:** cut to at most 1.2× budget, keeping every existing slot.

## Done when

- [ ] A plan-level template exists, and the file is within 1.2× budget.
- [ ] Both `[[verify]]` commands pass.

## Notes

- These files are untracked, so edit them in place in the main checkout. Edit only this item's anchored files.
- Keep every `[[RESULT …]]` slot, and use the marker grammar and the `observational` claim type in
  `tmp/cybernetic-harness/paper/00-README.md`.
- The working title is "A Cybernetic Harness for Spec'd Agent Work: Frontier Models Plan, Cheap Models Execute"; the
  thesis is golden path plus cybernetics. Use the status tags in `docs/whitepaper/data/mechanisms.toml`.
