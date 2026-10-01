+++
id = "gap-5d95a5"
kind = "gap"
title = "Companion report: trim §3, §7, §5 and §1 to budget, and fix the mapping-table misreadings and the zhang2025darwin venue"
status = "done"
triage = "verified"
severity = "p2"
goal = "whitepaper"
size = "M"
subsystem = ["companion"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "fce93aced"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:40, wk-companion-review's report on gap-a3031b)"
anchors = ["tmp/cybernetic-harness/companion-audit/E10-DRAFT.md", "tmp/cybernetic-harness/companion-audit/MAPPING-TABLE.md", "tmp/cybernetic-harness/companion-audit/data/bib-overrides.json", "tmp/cybernetic-harness/companion-audit/references.bib"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = ["gap-a3031b", "gap-cb86e4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q '&amp;' tmp/cybernetic-harness/companion-audit/references.bib && ! grep -q 'count empty replies as success (zhu2025establishing)' tmp/cybernetic-harness/companion-audit/MAPPING-TABLE.md"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "Companion trims and fixes (tmp, wk-companion-strict): §1 719->675/687, §3 615->406/412, §5 899->792/825, §7 1,293->1,085/1,100 words, cut detail moved to appendix blocks B.2, B.3, C.2, E.1 with no number, anchor, CL id, sha or date lost (script diff); MAPPING-TABLE lines 33 and 81 fixed and the MAST note closed; zhang2025darwin venue fixed via data/bib-overrides.json and the bib rebuilt (no &amp;); A4 labelled author-reported. Verify passes."
+++

## Problem

The E12 review (gap-a3031b) left several problems:

- §3 is at 1.49× its word budget, §7 at 1.17×, §5 at 1.09× and §1 at 1.05×.
- `MAPPING-TABLE.md` lines 33 and 81 repeat the breck and zhu misreadings that the review fixed in the draft. Its MAST-check note can be closed.
- The `zhang2025darwin` venue carries a stray `\&amp;`.
- Contribution A4's "AI-assisted design corpus" is author-reported (review M3) and needs that label.

## Why it matters

The companion report is the first publication candidate. Epic spec-f8d196.

## Where

The files in `anchors`. The bibliography is generated, so fix the venue in `data/bib-overrides.json` or `tmp/cybernetic-harness/tools/build_bibliography.py`, then rebuild. Don't hand-edit `references.bib`.

## Current state

The review records the problems in `06-REVIEW.md`, "E12 internal review".

## Plan

1. Trim each section to at most 1.0× its budget in `03-OUTLINE.md`. Move detail into the appendix material, and drop no number or anchor.
2. Fix the two mapping-table rows, and close the MAST note.
3. Fix the venue at its source and rebuild.
4. Label A4 as author-reported.

## Done when

- [ ] Every section is within budget, and the flagged errors are fixed.
- [ ] The `[[verify]]` command passes.
