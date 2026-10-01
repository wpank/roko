+++
id = "gap-11845c"
kind = "gap"
title = "docs/v3's descriptions of what cited papers say are unchecked; two harness-engineering sections checked so far were mostly invented"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "M"
subsystem = ["docs/v3", "tools/docs_integrity"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "97e716546"
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

- [x] At least 30 of the most-described works are checked and recorded, with the error rate and a recommendation.
- [x] The `[[verify]]` command passes.

## Notes

2026-10-01 (wk-rp-cite). Checked 45 works; the verdicts are in `tools/docs_integrity/content_audit.json`.

- **Ranking.** 167 arXiv works have descriptive units in docs/v3 (588 sentences, list items or table rows that state a
  method, result, number or taxonomy next to the work's ID). The 45 checked works are 45 of the top 57 and cover 58% of
  those units. Of the other 12, two were read in gap-0d996a and one has no description. The scan only sees arXiv IDs, and
  it undercounts depth files that name a work once as "Primary source".
- **Reading.** Full text from arxiv.org/html (latest version; the v1 abstract where the docs quoted v1 figures). Every
  descriptive unit was compared with the abstract, introduction, method and results, by reading and by searching for
  each claimed number and term.
- **Result.** 17 supported, 28 corrected, 0 removed. 62.2% of the works had at least one unsupported claim. 35.6% had a
  major one: an invented finding, number, mechanism or quotation, or another paper's result. Examples:
  - a "CoALA 9-step pipeline" that CoALA does not define;
  - Meta-Harness credited with SWE-bench Mobile's 6x gap and with a quotation it does not contain;
  - "41-87% of failures are coordination" where the paper gives 41-87% as the failure rate;
  - Memory Worth said to measure causal contribution, where the paper says its signal is associational, not causal;
  - HAL's 21,730 rollouts attached to a taxonomy paper;
  - SkillReducer described as tool pruning;
  - SiriuS's repaired trajectories described as "negative examples".
- **Errors do not thin out down the ranking.** Errors by rank: 12 of 15 in ranks 1-15, 8 of 14 in ranks 16-30, and
  8 of 16 in ranks 31-57.
- **Fixes.** Claims were rewritten with section references or labelled as Roko's own synthesis, in 42 docs/v3 files and
  14 docs/v1 copies. Wrong author labels in the same sentences were fixed too: Xie, Yuan, Li, Lange, Chen, Deshpande and
  Zhang were replaced by the real first authors.
- **Regression guard.** `citation_errata.json` gains 6 works with prose-only entries (FrugalGPT, CoALA, ACE, Lost in the
  Middle, Reflexion, Mesh Memory Protocol) and new phrases on 19 more, so `--prose` catches the removed claims if they
  return.
- **Not checked.** Venue and award claims that the paper text cannot confirm were left as they are: "ICML 2026
  Spotlight", "WWW 2026", "FSE 2026" and "Best Paper ACM CAIS 2026".
- **Recommendation.** Add a banner now, cut unchecked numbers to bare citations, and keep checking only the works the
  docs use as evidence. The unchecked 121 arXiv works, and the non-arXiv citations, should be assumed to have a similar
  error rate. Concretely:
  - Put a banner on REFERENCES.md and depth/39-references/ saying that descriptions are unverified unless the work is in
    content_audit.json.
  - Cut "Brief annotation" lines and other one-line descriptions of unchecked works to the bare citation, and drop
    unchecked numbers.
  - Keep reading in full only the works the docs cite as evidence for a design decision ("validates", "grounds",
    "demonstrates that"), starting with ranks 46-70 and the depth files that name a "Primary source".

