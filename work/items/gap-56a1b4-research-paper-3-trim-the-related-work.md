+++
id = "gap-56a1b4"
kind = "gap"
title = "Research paper §3: trim the related work to budget"
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
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W9-paper-workstream.md"
anchors = ["tmp/cybernetic-harness/paper/sections/03a-related-work.md", "tmp/cybernetic-harness/paper/sections/03b-related-work.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f tools/paperlint.py && python3 tools/paperlint.py --budget 1.2 --check-identifiers tmp/cybernetic-harness/paper/sections/03a-related-work.md tmp/cybernetic-harness/paper/sections/03b-related-work.md"
+++

## Problem

§3 runs about 1.7× its budget (W9: trim about 1.6k words).

## Why it matters

Length discipline for arXiv. The whitepaper's §10 (gap-ec516e) condenses from this section, so trim it first.
Epic spec-f8d196.

## Where

`tmp/cybernetic-harness/paper/sections/03a-related-work.md`, `tmp/cybernetic-harness/paper/sections/03b-related-work.md`.

## Current state

Stable content; over budget.

## Plan

1. **Cut:** merge overlapping paragraphs, and drop citations that don't bear on the thesis.
2. **Keep:** every `[@key]` that remains must resolve in `tmp/cybernetic-harness/paper/bibliography/references.bib`.
3. **Positioning:** add one paragraph positioning against orchestration tools, if it is missing (research note C3).

## Done when

- [ ] Both files are within 1.2× budget, and the `[[verify]]` command passes.

## Notes

- These files are untracked (`tmp/` is gitignored), so edit them in place in the main checkout. Only this item's files
  may be edited: other agents own the other section files at the same time.
- Keep the marker grammar in `tmp/cybernetic-harness/paper/00-README.md`. Never delete a `[[RESULT …]]` slot or turn it into a claim; results
  arrive later from S09 bundles.
- Use the thesis decided on 2026-09-29, "golden path plus cybernetics": frontier models plan, cheap models execute in
  parallel, gates and a whole-plan check verify, and feedback loops make it improve measurably. Status claims carry
  a commit.
