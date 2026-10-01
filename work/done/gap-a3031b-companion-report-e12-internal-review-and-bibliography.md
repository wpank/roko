+++
id = "gap-a3031b"
kind = "gap"
title = "Companion report E12: internal review and bibliography QA"
status = "done"
triage = "verified"
severity = "p2"
goal = "whitepaper"
size = "M"
subsystem = ["companion"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "cac54e574"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (14:47, wk-companion-e10's report on gap-652d05); tmp/cybernetic-harness/execution/checklist.json#E12"
anchors = ["tmp/cybernetic-harness/companion-audit/E10-DRAFT.md", "tmp/cybernetic-harness/companion-audit/05-RELATED-WORK-MAP.md", "tmp/cybernetic-harness/companion-audit/06-REVIEW.md", "tmp/cybernetic-harness/companion-audit/03-OUTLINE.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = ["gap-b409fa"], blocks = [], related = ["gap-652d05", "gap-cdd5f4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f tmp/cybernetic-harness/companion-audit/E10-DRAFT.md && ! grep -v '^>' tmp/cybernetic-harness/companion-audit/E10-DRAFT.md | grep -q '\\[\\[CHECK:' && test -f tmp/cybernetic-harness/companion-audit/05-RELATED-WORK-MAP.md && ! grep -q 'Neither appears in MAST' tmp/cybernetic-harness/companion-audit/05-RELATED-WORK-MAP.md"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "Companion E12 internal review (tmp, wk-companion-review): CHECK markers resolved against the papers; 9 misdescribed citations fixed; the rejected statements in 05 fixed; 03 synced; §4 2,261 -> 1,352 words and §6 1,796 -> 1,088 (within budget, no number or anchor dropped); bib QA: all 67 keys resolve, no internal paths in 400 entries, 34 DOIs match Crossref; paperlint --strict 114 -> 91 (identifiers and header 0). Recorded in 06-REVIEW.md 'E12 internal review'. Verify passes."
+++

## Problem

The companion checklist's step E12, "Companion internal review + bibliography QA" (M, after E10;
`execution/CHECKLIST.md:615`), has no work item, and E13 (posting the report) waits on it. Three things are known to
be waiting for it:

1. **Length.** The E10 draft (gap-652d05, about 12,600 words) is over its page budget in §4 and §6 (`03-OUTLINE.md`:
   §4 Method 2.5 pp, §6 RQ2 2 pp). wk-companion-e10 left the trim to this review.
2. **`[[CHECK: …]]` markers.** These are statements about a cited work that must be checked against the paper itself;
   the draft's marker legend assigns them to the E12 review.
3. **Related-work statements the review rejected.** `06-REVIEW.md` rejected two statements that
   `05-RELATED-WORK-MAP.md` still makes:
   - "Neither appears in MAST" (now `05:104`; review at `06:161`): MAST's FC1 is "System Design Issues";
   - the Ouroboros comparison, "disagrees by two orders of magnitude" (`05:54-55`; review row 21 at `06:306`): the
     units differ (a share of self-modification commits against a share of Rust lines).

## Why it matters

The companion report is the first publication candidate (epic spec-f8d196). A statement a reviewer has already
rejected, or an unchecked claim about someone else's paper, is the cheapest way to lose credibility.

## Where

`tmp/cybernetic-harness/companion-audit/`: `E10-DRAFT.md`, `05-RELATED-WORK-MAP.md`, `06-REVIEW.md` (the review's
issues), `03-OUTLINE.md` (page budgets), `BIB-QA.md` and `references.bib` (bibliography).

## Current state

Checked in MAIN on 2026-09-29:
- the draft has two `[[CHECK:` markers outside the legend (MAST's failure-mode codes, and the Eichorn and Yankauer
  reference-error rate);
- both rejected statements are still in `05`; its quotation-accuracy baseline (the review's claim-support issue) has
  already been reworded (`05:127`);
- gap-b409fa (filling the E1 numbers and fixing the source errors) runs first.

## Plan

1. Check each `[[CHECK: …]]` statement against the cited paper, then fix or drop it.
2. Correct the two rejected statements in `05`, and wherever the draft repeats them.
3. Trim §4 and §6 to the outline's budget.
4. Bibliography QA: every key resolves, the venues are right, and no field carries an internal path.
5. Record the verdict and the remaining issues at the end of `06-REVIEW.md` or in a short `E12-REVIEW.md`.

## Done when

- [ ] No `[[CHECK:` marker remains outside the legend, and the two rejected statements are fixed.
- [ ] §4 and §6 are within the outline's page budget.
- [ ] Bibliography QA is recorded.
- [ ] The `[[verify]]` command passes.

## Notes

- Untracked files: edit them in place in the main checkout, after gap-b409fa has finished with `E10-DRAFT.md`.
- The `[[E2:…]]` and `[[E3:…]]` placeholders stay until the human raters report.
