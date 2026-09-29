+++
id = "gap-b4e597"
kind = "gap"
title = "Research paper: apply the D13 prior-art edits, since C1.23 is false as worded"
status = "done"
triage = "verified"
severity = "p1"
goal = "whitepaper"
size = "M"
subsystem = ["paper"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "860b10a88"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:40, wk-prior-art's report on gap-bb619d)"
anchors = ["tmp/cybernetic-harness/paper/sections/01-introduction.md", "tmp/cybernetic-harness/paper/sections/03a-related-work.md", "tmp/cybernetic-harness/paper/sections/03b-related-work.md", "tmp/cybernetic-harness/paper/sections/08-discussion.md", "tmp/cybernetic-harness/paper/sections/B-spec-standard.md", "tmp/cybernetic-harness/paper/OUTLINE.md", "tmp/cybernetic-harness/paper/REVIEW-SKELETON.md", "tmp/cybernetic-harness/paper/bibliography/"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = ["gap-bb619d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'We know of no harness that estimates how often its passes are still' tmp/cybernetic-harness/paper/sections/01-introduction.md && grep -q 'kill-search D13 run 2026-09-29' tmp/cybernetic-harness/paper/sections/03a-related-work.md && grep -q 'kill-search D13 run 2026-09-29' tmp/cybernetic-harness/paper/sections/03b-related-work.md && python3 tools/paperlint.py --budget 1.2 --check-identifiers tmp/cybernetic-harness/paper/sections/01-introduction.md tmp/cybernetic-harness/paper/sections/03a-related-work.md tmp/cybernetic-harness/paper/sections/03b-related-work.md tmp/cybernetic-harness/paper/sections/08-discussion.md tmp/cybernetic-harness/paper/sections/B-spec-standard.md"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "D13 prior-art edits applied (tmp, wk-rp-priorart): E1-E17 in §1, §3a, §3b, §8, App B, OUTLINE, REVIEW-SKELETON and the specs/research notes (S07, S04, P-positioning, M4, M2); E18 in §5.3, §5.4 and §6.6. C1.23 now claims M4 as a combination citing PinSieve and Bounded Loops; C3b.6 keyed to FIRE. 33 records in bibliography/new-refs-D13.jsonl, re-checked with refcheck; bib rebuilt (286 cited keys, 0 unknown); doc citations rebuilt; CLAIMS-EVIDENCE regenerated (429 claims). paperlint --budget 1.2 --check-identifiers: 18 files clean. PRIOR-ART-D13.md §4 marked applied."
+++

## Problem

The D13 kill-search (`paper/PRIOR-ART-D13.md`, gap-bb619d) found that C1.23's general form is false. §1.2 says "we know of no harness that estimates how often its passes are still wrong behind its gates", but PinSieve (arXiv 2608.24040) audit-samples its auto-passes and estimates the miss rate by IPW. Bounded Loops (2609.27871) measures a harness's gate false-accept rate. The search also narrows the M1, M2 and routing claims, and C3b.6 and O9.

## Why it matters

A factual error in the introduction would sink the paper in review. Epic spec-f8d196.

## Where

Section 4 of `PRIOR-ART-D13.md` lists each change, with file and line.

- E1–E11 edit §1.2, §3.1–§3.7, §8.5 and Appendix B.
- E12 edits OUTLINE, E13 REVIEW-SKELETON, and E14 the status lines.
- E15 edits the specs and research notes: S04, P-positioning, M4, M2.
- E17 adds the bib records.
- E18 is optional.
- E16, the whitepaper edit, is a separate item.

## Current state

None of the edits is applied.

## Plan

1. Apply E1–E15 and E17, and E18 where it fits the budget.
2. Add the 33 bib records through the paper bibliography's source files, then rebuild with `build_main_bib.py`.
3. Claim M4 as the combination (E1, E7), citing PinSieve and Bounded Loops.
4. Keep every `[[RESULT …]]` slot, and keep each file within 1.2× budget.

## Done when

- [ ] E1–E15 and E17 are applied, and C1.23 is reworded.
- [ ] The `[[verify]]` command passes.

## Notes

- **From wk-prior-art (gap-bb619d, 2026-09-29):** the edits keep the "(search of 2026-09-29)" dating. The submission-week re-run of the listing sweep (Q13–Q15 and Q17) and the unread Li 2026 lead are tracked in gap-85f86a, held until submission week.
