+++
id = "gap-aad48c"
kind = "gap"
title = "Research paper: align the outline, abstract, introduction and conclusion with the golden-path thesis"
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
anchors = ["tmp/cybernetic-harness/paper/OUTLINE.md", "tmp/cybernetic-harness/paper/sections/00-abstract.md", "tmp/cybernetic-harness/paper/sections/01-introduction.md", "tmp/cybernetic-harness/paper/sections/10-conclusion.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f tools/paperlint.py && python3 tools/paperlint.py --budget 1.2 --check-identifiers tmp/cybernetic-harness/paper/sections/00-abstract.md tmp/cybernetic-harness/paper/sections/01-introduction.md tmp/cybernetic-harness/paper/sections/10-conclusion.md"

[[verify]]
command = "grep -qi 'golden path' tmp/cybernetic-harness/paper/OUTLINE.md && grep -qi 'golden path' tmp/cybernetic-harness/paper/sections/01-introduction.md"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "Research paper outline, abstract, introduction and conclusion rewritten around the golden-path thesis (untracked tmp/cybernetic-harness/paper); within 1.2x budget; both verifies pass with the merged paperlint."
+++

## Problem

The research draft is organised around M1–M4 and tests cheap execution of single, pre-written tasks. It barely
mentions planning and never mentions integration, which is the loop the tldr and the whitepaper centre on
(assessment W9, finding 2). Its abstract, introduction and conclusion run 1.4–1.6× their word budgets.

## Why it matters

The whitepaper and the research paper must tell one story. Epic spec-f8d196.

## Where

`tmp/cybernetic-harness/paper/OUTLINE.md` (contributions, title), `tmp/cybernetic-harness/paper/sections/00-abstract.md`, `tmp/cybernetic-harness/paper/sections/01-introduction.md`, `tmp/cybernetic-harness/paper/sections/10-conclusion.md`.

## Current state

The working title is "A Cybernetic Agent Harness: Making Cheap Models Dependable through Self-Regulation". The
whitepaper's is "Roko: a Cybernetic Harness for Spec'd Agent Work". The evaluation scope was decided on 2026-09-29:
single tasks plus a plan-level slice.

## Plan

1. **Contributions:** reframe them in OUTLINE.md around the golden path plus the cybernetic loops, keeping M1–M4 as
   the mechanisms of improvement. Record the plan-level slice as exploratory.
2. **Introduction:** add a field vignette (evidence/field/CASES.md CASE-005 or CASE-006).
3. **Trim:** cut the abstract, introduction and conclusion to at most 1.2× budget. Keep every `[[RESULT]]` slot.
4. **Identifiers:** mark code identifiers that don't exist yet as designed (`[[AS-BUILT: …]]`).

## Done when

- [ ] OUTLINE.md and §1 state the golden path and the cybernetic loops as the thesis.
- [ ] The three sections are within 1.2× budget, and no untagged identifier is missing from the code.
- [ ] Both `[[verify]]` commands pass.

## Notes

- These files are untracked (`tmp/` is gitignored), so edit them in place in the main checkout. Only this item's files
  may be edited: other agents own the other section files at the same time.
- Keep the marker grammar in `tmp/cybernetic-harness/paper/00-README.md`. Never delete a `[[RESULT …]]` slot or turn it into a claim; results
  arrive later from S09 bundles.
- Use the thesis decided on 2026-09-29, "golden path plus cybernetics": frontier models plan, cheap models execute in
  parallel, gates and a whole-plan check verify, and feedback loops make it improve measurably. Status claims carry
  a commit.
