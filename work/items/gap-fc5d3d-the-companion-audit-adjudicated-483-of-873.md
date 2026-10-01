+++
id = "gap-fc5d3d"
kind = "gap"
title = "The companion audit adjudicated 483 of 873 cited works, leaving 390 unchecked, and docs/v1's 103 known errata locations are untouched"
status = "open"
triage = "unverified"
severity = "p3"
goal = "release"
size = "L"
subsystem = ["docs/v1", "tools/docs_integrity"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-rp-cite's report)"
anchors = ["tools/docs_integrity/citation_errata.json", "tools/docs_integrity/check_citation_errata.py", "docs/v1/"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["gap-b23ebd", "gap-212b75"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'docs/v1' tools/docs_integrity/check_citation_errata.py && python3 tools/docs_integrity/check_citation_errata.py"
+++

## Problem

gap-b23ebd fixed docs/v3 against the companion audit's adjudicated errata. Two parts remain (wk-rp-cite):

- **Unchecked works.** The audit adjudicated 483 of the 873 works the docs cite, so 390 are unchecked and may hold more errors.
- **docs/v1.** It has 103 known errata locations that nobody has touched, and the checker covers docs/v3 only.

## Why it matters

Release: the docs are public. p3, because docs/v1 is deprecated and the unchecked works are the ones cited less often.

## Where

`citation_errata.json` and the checker. The adjudication data is in the companion audit (`tmp/cybernetic-harness/companion-audit/`).

## Plan

1. Adjudicate the remaining 390 works, or record which ones are out of scope, and add their errata.
2. Extend the checker to docs/v1 (or mark docs/v1 as unmaintained, with a banner, and exclude it explicitly), and fix or annotate its 103 locations.

## Done when

- [ ] Every cited work is adjudicated or explicitly out of scope, and docs/v1 is checked or explicitly excluded.
- [ ] The `[[verify]]` command passes (it requires the checker to cover docs/v1; adjust it if v1 is excluded instead).
