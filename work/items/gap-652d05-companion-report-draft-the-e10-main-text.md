+++
id = "gap-652d05"
kind = "gap"
title = "Companion report: draft the E10 main text with E1 number markers"
status = "done"
triage = "verified"
severity = "p2"
goal = "whitepaper"
size = "M"
subsystem = ["companion"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "847384ae2"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e18"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W9-paper-workstream.md"
anchors = ["tmp/cybernetic-harness/companion-audit/"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "ls tmp/cybernetic-harness/companion-audit/E10-*.md >/dev/null 2>&1 && grep -q '\\[\\[E1:' tmp/cybernetic-harness/companion-audit/E10-*.md"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "tmp/cybernetic-harness/companion-audit/E10-DRAFT.md (untracked) drafts the companion report's main text: all ten outline sections, Tables 1-6 and the after-the-audit box, with every number as an [[E1: …]] marker (362) and E2/E3 rater results as placeholders. Review fixes applied (headline 0 of 65 mechanisms, windows named for every false-green figure). About 9,600 words; §4 and §6 run over, left for the E12 review. Verify passes."
+++

## Problem

The companion report's main text (checklist E10) is not drafted (W9 CW3).

## Why it matters

The companion is the first publication candidate. It needs raters for E2 and E3, but its text can be written now,
with `[[E1: …]]` markers that the re-derivation fills in. Epic spec-f8d196.

## Where

`tmp/cybernetic-harness/companion-audit/`: follow its `03-OUTLINE.md`. New file `E10-DRAFT.md`.

## Current state

The outline and the evaluation plan exist. The worksheets for E2 (97 rows) and E3 (40 claims) are ready for the
raters.

## Plan

1. **Draft:** follow 03-OUTLINE.md, and mark every number `[[E1: <metric>]]` until the re-derivation is done.
2. **Placeholders:** keep the E2 and E3 results as `[[E2:…]]` and `[[E3:…]]` placeholders; raters are still needed.

## Done when

- [ ] `E10-DRAFT.md` covers the outline's sections, with markers for every number.
- [ ] The `[[verify]]` command passes.
