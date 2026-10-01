+++
id = "gap-fc5d3d"
kind = "gap"
title = "The companion audit adjudicated 483 of 873 cited works, leaving 390 unchecked, and docs/v1's 103 known errata locations are untouched"
status = "done"
triage = "verified"
severity = "p3"
goal = "release"
size = "L"
subsystem = ["docs/v1", "tools/docs_integrity"]
created = 2026-09-30
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "e5da8676e"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-rp-cite's report)"
anchors = ["tools/docs_integrity/citation_errata.json", "tools/docs_integrity/check_citation_errata.py", "docs/v1/"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["gap-b23ebd", "gap-212b75"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'docs/v1' tools/docs_integrity/check_citation_errata.py && python3 tools/docs_integrity/check_citation_errata.py"

[closed]
at = 2026-10-01
commit = "e5da8676e"
evidence = "check_citation_errata.py covers docs/v1 (130 known errata at a4e175c9c -> 0, prose 259 -> 0); all 412 unadjudicated works checked against registry records: 356 clean, 52 errata added and fixed, 4 out of scope, listed in citation_errata.json 'checked'; verify and 13 tests pass"
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

## Notes

- 2026-10-01 (wk-rp-cite), **docs/v1.** The checker covers docs/v1 by default. At `a4e175c9c`, docs/v1 had 130 known
  errata (259 with `--prose`) in 53 files. Titles, authors, IDs and years are now corrected, and 21 entries for
  fabricated works are removed. One source is annotated instead: the BENCHMARKS composite score was attributed to
  "Sheppert, 2026", which no registry lists. On DataCite, ToolCacheAgent turned out to be ToolCaching
  (arXiv:2601.15335) and SAMULE is arXiv:2509.20562, so the two docs/v3 ToolCaching entries that gap-b23ebd removed are
  restored.
- **Unchecked works, batch 1 (all of them).** The audit's automated pass had registry records for the 412 works it
  never adjudicated: its population of 873 less 483, plus merged and economy keys. The records come from DataCite for
  arXiv IDs and Crossref, OpenAlex or DataCite for titles. Each was compared on title, first author, author list, year
  and ID. I read all 108 suspects by hand and re-checked the person-name mismatches live on DataCite and Crossref.
  - **356 clean.** Among them are 19 books and classics whose registry record is a review or omits the subtitle, and 39
    inline arXiv mentions whose context matches the paper.
  - **52 errata:** 36 placeholder authors, 15 wrong authors or titles and 1 wrong ID. They are added to
    `citation_errata.json` and fixed, with 150 citation fixes across docs/v3 and docs/v1.
  - **4 out of scope:** 2 are cited only in docs/v2-depth, which neither checked tree contains. 2 are anonymous
    OpenReview citations whose published version is a later record, so the attribution is ambiguous.
  - The manifest's `checked` section lists every clean and out-of-scope key, with the method.
- **Limits.** Claim support (whether a paper says what the docs say) is not assessed, as in the audit. The population is
  the audit's, so citations added after 2026-09-28 are covered only when they repeat a known erratum.
