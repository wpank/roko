+++
id = "gap-11845c"
kind = "gap"
title = "docs/v3's descriptions of what cited papers say are unchecked; two harness-engineering sections checked so far were mostly invented"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
size = "M"
subsystem = ["docs/v3", "tools/docs_integrity"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-rp-cite's reports on gap-785a4f and gap-0d996a)"
anchors = ["docs/v3/", "tools/docs_integrity/citation_errata.json"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["gap-785a4f", "gap-0d996a", "gap-fc5d3d", "gap-212b75"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f tools/docs_integrity/content_audit.json && python3 -c \"import json;d=json.load(open('tools/docs_integrity/content_audit.json'));assert len(d['works'])>=30\" && python3 tools/docs_integrity/check_citation_errata.py --prose"
+++

## Problem

The citation checker verifies titles, authors, ids and years, not whether a paper says what the docs claim it says. gap-212b75, gap-785a4f and gap-0d996a read papers in full and found invented content each time: routing results with no source, "six harness principles" that Meta-Harness never states, and in harness-engineering.md §3–§6 taxonomies, metrics and a mechanism table that none of four papers contain (wk-rp-cite). docs/v3 cites 296 works across 905 files; the rest of those descriptions have never been read against their papers.

## Why it matters

Release: the docs are public, and the papers point readers at the repository. Invented research claims are a credibility risk.

## Plan

1. Rank the cited works by how much the docs say about them (sentences that describe a work's method, results, taxonomy or numbers, not bare mentions).
2. Read the top 30 or more in full and check each description. Rewrite unsupported claims, with section references, or mark Roko's own synthesis as such. Add phrase entries to `citation_errata.json` so `--prose` catches them if they return.
3. Record each checked work, its verdict (supported, corrected, or removed) and the error rate in `tools/docs_integrity/content_audit.json`.
4. From the error rate, recommend what to do with the unchecked rest: continue, add a banner, or cut the descriptions down to bare citations.

## Done when

- [ ] At least 30 of the most-described works are checked and recorded, with the error rate and a recommendation.
- [ ] The `[[verify]]` command passes.
