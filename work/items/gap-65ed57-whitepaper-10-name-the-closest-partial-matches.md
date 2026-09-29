+++
id = "gap-65ed57"
kind = "gap"
title = "Whitepaper §10: name the closest partial matches from the D13 prior-art search"
status = "done"
triage = "verified"
severity = "p2"
goal = "whitepaper"
size = "S"
subsystem = ["paper"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "ebffa7842"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:40, wk-prior-art's report on gap-bb619d, edit E16)"
anchors = ["docs/whitepaper/10-related-work.md", "docs/whitepaper/references.bib"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = [], blocks = [], related = ["gap-bb619d", "gap-ec516e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'gao2026pinsieve' docs/whitepaper/10-related-work.md && grep -q '{gao2026pinsieve,' docs/whitepaper/references.bib && python3 tools/paperlint.py --strict --require-status reviewed docs/whitepaper/*.md"

[closed]
at = 2026-09-29
commit = "ebffa7842"
evidence = "10-related-work.md names the four closest partial matches (gao2026pinsieve, bhardwaj2026bounded, li2026rsirouter, xu2026replaylens), each added to references.bib after refcheck and an arxiv.org abstract read; §10 stays at 533 words (1.07x); paperlint --strict --require-status reviewed docs/whitepaper/*.md passes (14 files clean) and pandoc citeproc renders §10 without warnings"
+++

## Problem

The D13 search (`tmp/cybernetic-harness/paper/PRIOR-ART-D13.md`, E16) found that the whitepaper's three-way combination claim in §10 survives. It proposes naming the closest partial matches after the claim: PinSieve, Bounded Loops, RSI-Router and ReplayLens.

## Why it matters

Readers trust a novelty claim more when its nearest neighbours are named. Epic spec-ce1484.

## Where

`docs/whitepaper/10-related-work.md`, lines 46–48, and `docs/whitepaper/references.bib`. The keys and the full bibliographic data are in the D13 file.

## Current state

The whitepaper is reviewed (gap-8d2c79) at 1.10× its total budget, so the addition must be offset by cuts.

## Plan

1. Add the E16 sentence, and add the four entries to `references.bib`, verified against their sources.
2. Trim §10 by the same number of words.
3. Keep `Status: reviewed`, and pass `paperlint --strict`.

## Done when

- [ ] §10 names the four matches, and §10 stays within budget.
- [ ] The `[[verify]]` command passes.
