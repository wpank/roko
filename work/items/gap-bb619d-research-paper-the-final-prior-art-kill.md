+++
id = "gap-bb619d"
kind = "gap"
title = "Research paper: the final prior-art kill-search (Track D13) before submission"
status = "open"
triage = "unverified"
severity = "p2"
goal = "whitepaper"
size = "M"
hold = "Runs in submission week (paper/OUTLINE.md, Track D13)"
subsystem = ["paper"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (15:04); tmp/cybernetic-harness/paper/OUTLINE.md:113"
anchors = ["tmp/cybernetic-harness/paper/OUTLINE.md", "tmp/cybernetic-harness/paper/REVIEW-SKELETON.md", "tmp/cybernetic-harness/paper/sections/08-discussion.md", "tmp/cybernetic-harness/paper/sections/B-spec-standard.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = ["gap-56a1b4", "gap-ec516e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f tmp/cybernetic-harness/paper/PRIOR-ART-D13.md && grep -q '^Verdict:' tmp/cybernetic-harness/paper/PRIOR-ART-D13.md"
+++

## Problem

The research paper's outline plans a last prior-art "kill-search" in submission week (`paper/OUTLINE.md:113`, Track
D13; assessment W9 lists it for §3). Two novelty claims are gated on it (`paper/REVIEW-SKELETON.md`):

- **O7:** "What is new here is the measurement: an operational influence metric applied to every loop of an LLM
  harness" (`08-discussion.md:159-160`);
- **O9:** "The novelty assessment in S07 §0.1 found no prior system that does all three" (`B-spec-standard.md:21-22`),
  which rests on an internal spec search.

No work item tracks the search, so nothing makes sure it runs before submission.

## Why it matters

An unhedged novelty claim that a reviewer can refute with one citation costs the paper its credibility. Epic
spec-f8d196.

## Where

- `tmp/cybernetic-harness/paper/OUTLINE.md` (Track D13) and `REVIEW-SKELETON.md` (O7, O9).
- The claims' sections: `sections/08-discussion.md` and `sections/B-spec-standard.md`.
- Output: `tmp/cybernetic-harness/paper/PRIOR-ART-D13.md` (new).

## Current state

The search has not run, and it should not run before submission week. The claims are to be hedged ("we know of no …")
until it does.

## Plan

1. In submission week, search arXiv, OpenReview, the ACL Anthology, Semantic Scholar and GitHub for work that combines
   the elements each novelty claim names. Record the queries, dates, hits and a verdict per claim in
   `PRIOR-ART-D13.md`.
2. Revise O7 and O9, and any other novelty sentence, to match: keep, hedge, or cite the prior work.
3. Add any new related work to §3 and the bibliography.

## Done when

- [ ] `PRIOR-ART-D13.md` records the search and ends with a `Verdict:` line.
- [ ] Every novelty claim in the paper agrees with it.
- [ ] The `[[verify]]` command passes.

## Notes

- Untracked files: edit them in place in the main checkout.
- Check the whitepaper's related work (§10, gap-ec516e) against the same results.
