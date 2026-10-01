+++
id = "gap-6e0a95"
kind = "gap"
title = "Whitepaper: no claim it makes about a cited work has been checked against that work's full text"
status = "open"
triage = "unverified"
severity = "p1"
goal = "whitepaper"
size = "M"
subsystem = ["docs/whitepaper"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (coordinator, after gap-11845c found unsupported claims in 62% of the docs/v3 works read in full)"
anchors = ["docs/whitepaper/10-related-work.md", "docs/whitepaper/references.bib", "docs/whitepaper/data/"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = [], blocks = [], related = ["gap-11845c", "gap-1f72ac", "gap-08d9b2", "gap-8117a8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c \"import json;d=json.load(open('docs/whitepaper/data/citation-content-audit.json'));assert d['complete'] is True and len(d['works'])>=40\" && python3 tools/paperlint.py --strict docs/whitepaper/0*.md docs/whitepaper/10-related-work.md"
+++

## Problem

The whitepaper cites 44 works (27 in §10). The companion audit and the citation checker verified their metadata (titles, authors, ids, years), but nobody has read the cited papers against what the whitepaper says about them. In docs/v3, agent-written descriptions turned out unsupported for 62% of the works read in full (gap-11845c), and the whitepaper was drafted the same way.

## Why it matters

The whitepaper is about to be tagged (gap-8117a8). A misdescribed related work is the kind of error a reader checks first.

## Plan

1. For every work the whitepaper cites, read the full text (arXiv HTML or the publisher's version) and check each sentence that describes it: method, results, numbers, positioning.
2. Fix what the paper doesn't support, with a section reference where useful, and keep the whitepaper's claims about Roko unchanged.
3. Record each work, its verdict (supported, corrected, removed), the sentence(s) checked and the evidence in `docs/whitepaper/data/citation-content-audit.json` with `"complete": true`.
4. `paperlint --strict` must still pass, and the status matrix must not change.

## Done when

- [ ] Every cited work is checked and recorded.
- [ ] The `[[verify]]` command passes.
