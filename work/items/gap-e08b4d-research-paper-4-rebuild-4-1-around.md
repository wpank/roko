+++
id = "gap-e08b4d"
kind = "gap"
title = "Research paper §4: rebuild §4.1 around the control stack and the golden-path loop, and tag designed identifiers"
status = "open"
triage = "verified"
severity = "p1"
goal = "whitepaper"
size = "M"
subsystem = ["paper"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e18"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W9-paper-workstream.md"
anchors = ["tmp/cybernetic-harness/paper/sections/04-system.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f tools/paperlint.py && python3 tools/paperlint.py --budget 1.2 --check-identifiers tmp/cybernetic-harness/paper/sections/04-system.md"
+++

## Problem

§4 (6.6k words, about 2× budget) is organised by mechanism (M3, M4, M2, M1, S07). It names 8 identifiers that don't
exist at HEAD: `AttemptKey`, `LoopSpec`, `HarnessParams`, `audit_select`, `cost_source`, `AuditRecord`,
`DecisionRecord` and `ExposureRecord` (W9 finding 3).

## Why it matters

§4 is the system description readers check against the code, and the companion report criticises exactly this
pattern of phantom identifiers. Epic spec-f8d196.

## Where

`tmp/cybernetic-harness/paper/sections/04-system.md`. Sources: `tmp/cybernetic-harness/tldr/02-HOW-IT-WORKS.md`, `tldr/04`, specs S01–S07.

## Current state

The draft is in the as-designed voice, and the identifiers are untagged.

## Plan

1. **§4.1:** rewrite it as the control stack (feedforward, feedback, second order) plus the golden-path loop (tldr/04's
   11 steps), with a status tag at a commit on each step.
2. **Mechanism subsections:** keep them, and cite spec § and version.
3. **Identifiers:** wrap every not-yet-built identifier in `[[AS-BUILT: …]]`, naming the work item that builds it
   (manifest.json).
4. **Trim:** cut to at most 1.2× budget.

## Done when

- [ ] §4.1 follows the control stack and the golden-path loop.
- [ ] No untagged identifier is missing from `crates/`.
- [ ] The file is within 1.2× budget, and the `[[verify]]` command passes.

## Notes

- These files are untracked (`tmp/` is gitignored), so edit them in place in the main checkout. Only this item's files
  may be edited: other agents own the other section files at the same time.
- Keep the marker grammar in `tmp/cybernetic-harness/paper/00-README.md`. Never delete a `[[RESULT …]]` slot or turn it into a claim; results
  arrive later from S09 bundles.
- Use the thesis decided on 2026-09-29, "golden path plus cybernetics": frontier models plan, cheap models execute in
  parallel, gates and a whole-plan check verify, and feedback loops make it improve measurably. Status claims carry
  a commit.
