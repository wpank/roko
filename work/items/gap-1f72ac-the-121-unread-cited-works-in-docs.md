+++
id = "gap-1f72ac"
kind = "gap"
title = "The ~121 unread cited works in docs/v3 likely carry unsupported claims at the audited 62% rate: banner the references, cut unchecked annotations to bare citations"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
size = "M"
subsystem = ["docs/v3", "docs/v1", "tools/docs_integrity"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-rp-cite's gap-11845c result: 28 of 45 audited works corrected, 62.2%)"
anchors = ["docs/v3/REFERENCES.md", "tools/docs_integrity/content_audit.json", "tools/docs_integrity/check_citation_errata.py"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["gap-11845c", "gap-0d996a", "gap-785a4f", "gap-fc5d3d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c \"import json;d=json.load(open('tools/docs_integrity/content_audit.json'));assert d.get('unchecked_annotations_cut') is True\" && grep -q 'not been checked against' docs/v3/REFERENCES.md && python3 tools/docs_integrity/check_citation_errata.py --prose"
+++

## Problem

gap-11845c read the 45 most-described cited works in full: 28 (62.2%) had a claim their paper does not support, 16 (35.6%) a major one, and the rate did not fall with rank (12/15, 8/14, 8/16 by band). The other ~121 cited arXiv works were not read, and their descriptions in docs/v3 and docs/v1 are likely wrong at a similar rate (wk-rp-cite).

## Why it matters

Release: the docs are public, and the papers point readers at the repository. Reading every remaining work in full costs far more than the descriptions are worth.

## Plan

1. Add a short banner to `docs/v3/REFERENCES.md` and the docs/v1 reference list saying which works' descriptions were checked against the papers (the `content_audit.json` list) and that the rest have not been checked against the papers.
2. Script the cut: for every cited work not in `content_audit.json`, reduce one-line annotations to the bare citation and drop numbers attributed to it. Keep the citation itself (title, authors, id, year).
3. Keep full reads only for works the docs cite as evidence for a design decision. List them in the item notes, and read them in a follow-up if there are any.
4. Record `"unchecked_annotations_cut": true` and the counts in `content_audit.json`. The checker stays clean with `--prose`.

## Done when

- [ ] Unchecked annotations are bare citations, the banner is in place, and the audit file records the cut.
- [ ] The `[[verify]]` command passes.
