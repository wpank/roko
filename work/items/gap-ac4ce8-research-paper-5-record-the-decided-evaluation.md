+++
id = "gap-ac4ce8"
kind = "gap"
title = "Research paper §5: record the decided evaluation scope and trim the protocol to budget"
status = "done"
triage = "verified"
severity = "p1"
goal = "whitepaper"
size = "M"
subsystem = ["paper"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "dc371b1ae"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e18"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W9-paper-workstream.md"
anchors = ["tmp/cybernetic-harness/paper/sections/05-evaluation-protocol.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f tools/paperlint.py && python3 tools/paperlint.py --budget 1.2 --check-identifiers tmp/cybernetic-harness/paper/sections/05-evaluation-protocol.md"

[[verify]]
command = "grep -qi 'plan-level slice' tmp/cybernetic-harness/paper/sections/05-evaluation-protocol.md"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "Research paper §5 records the decided scope (single tasks plus the exploratory plan-level slice, S09 arm ids) at 1.12x budget; both verifies pass."
+++

## Problem

§5 predates the 2026-09-29 decisions (W9 PW05):
- the evaluation covers single tasks plus a small plan-level slice;
- the Claude Code arm runs on the subscription (D42);
- D4 names the benchmark ViabilityBench at `benchmarks/viabilitybench/`.

The arms in tldr/04, S09 and the E12 items differ, and §5 is 1.5× its budget.

## Why it matters

§5 is the pre-registration text. It must be settled before the D2 lock (PW11). Epic spec-f8d196.

## Where

`tmp/cybernetic-harness/paper/sections/05-evaluation-protocol.md`. Sources: `tmp/cybernetic-harness/specs/S08-*.md`, `S09-*.md`, the E12 items, and
`tmp/cybernetic-harness/DECISIONS.md`.

## Current state

No text covers the plan-level slice, and the arm names are inconsistent.

## Plan

1. **Arms:** reconcile them into one table (S09's arm ids, with tldr/04's plain names beside them).
2. **Plan-level slice:** add it as an exploratory outcome, not confirmatory: makespan, and cost per verified feature.
3. **Decisions:** record D4 and D42.
4. **Trim:** cut to at most 1.2× budget. Keep every `[[RESULT]]` slot.

## Done when

- [ ] The arms are consistent with S09, and the slice is described as exploratory.
- [ ] The file is within 1.2× budget, and both `[[verify]]` commands pass.

## Notes

- These files are untracked (`tmp/` is gitignored), so edit them in place in the main checkout. Only this item's files
  may be edited: other agents own the other section files at the same time.
- Keep the marker grammar in `tmp/cybernetic-harness/paper/00-README.md`. Never delete a `[[RESULT …]]` slot or turn it into a claim; results
  arrive later from S09 bundles.
- Use the thesis decided on 2026-09-29, "golden path plus cybernetics": frontier models plan, cheap models execute in
  parallel, gates and a whole-plan check verify, and feedback loops make it improve measurably. Status claims carry
  a commit.
