+++
id = "gap-212b75"
kind = "gap"
title = "docs/v3 prose still describes routing papers (BEST-Route, xRouter, Router-R1) under invented titles, and the CLEAR-framework sections cite a fabricated work"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
size = "M"
subsystem = ["docs/v3"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "1637ff138"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-rp-cite's report)"
anchors = ["docs/v3/20-GATEWAY.md", "docs/v3/depth/20-gateway/06-routing-research.md", "docs/v3/depth/06-composition/distributed-context-engineering.md", "docs/v3/depth/06-composition/token-budget-management.md", "tools/docs_integrity/citation_errata.json"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["gap-b23ebd", "gap-052646", "gap-fc5d3d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 tools/docs_integrity/check_citation_errata.py --prose docs/v3/20-GATEWAY.md docs/v3/depth/20-gateway/06-routing-research.md docs/v3/depth/06-composition/distributed-context-engineering.md docs/v3/depth/06-composition/token-budget-management.md"

[closed]
at = 2026-09-30
commit = "1637ff138"
evidence = "check_citation_errata.py --prose (new) is clean on the four verify files and all of docs/v3 (73 errata at 286c5e53a, 20 in the verify files); routing prose rewritten from the real abstracts; CLEAR re-attributed to Mehta 2025 (arXiv:2511.14136); 13 unit tests pass"
+++

## Problem

gap-b23ebd corrected the citation lines in docs/v3. The prose that describes the works still uses titles that were invented (wk-rp-cite):

- **The routing papers.** BEST-Route, xRouter and Router-R1 are described under invented titles in `docs/v3/20-GATEWAY.md` and `docs/v3/depth/20-gateway/06-routing-research.md` §2–§6. The same works are mentioned in `depth/05-agent/dual-process-routing.md`, `depth/05-agent/11-dual-process-routing.md` and `REFERENCES.md`.
- **The CLEAR framework.** Its sections in `depth/06-composition/` (`distributed-context-engineering.md`, `token-budget-management.md`) cite a fabricated work.

## Why it matters

Release: invented titles and fabricated sources in public documentation undermine every other claim in it.

## Where

The files above. The checker's data file is `tools/docs_integrity/citation_errata.json`.

## Plan

1. Review each passage against the real papers: correct the titles and the claims, and rewrite or remove the CLEAR-framework sections that rest on the fabricated work.
2. `citation_errata.json` already has entries for these works, and the checker passes, because it checks citation lines, not prose. Give the checker a prose mode (`--prose FILE...`) that flags a work described under a title other than its adjudicated one, or a work marked fabricated.

## Done when

- [ ] No docs/v3 prose names or describes a work that doesn't exist, or under a title it doesn't have.
- [ ] The `[[verify]]` command passes.

## Notes

- 2026-09-30 (wk-rp-cite): reread against the real abstracts (DataCite, arXiv). The routing prose in `20-GATEWAY.md`,
  `depth/20-gateway/06-routing-research.md`, `depth/05-agent/*dual-process-routing.md` and
  `depth/08-learning/self-improvement-frameworks.md` now describes Router-R1, xRouter, IRT-Router, BEST-Route and
  cascade routing as their papers do, and the unsupported numbers are gone. CLEAR is real (Mehta 2025,
  arXiv:2511.14136); the audit's I1 rested on the invented title, so the sections now cite it and its manifest record is
  I0.
- `check_citation_errata.py --prose` checks each Markdown section that names a work for its wrong titles, author lists
  and the phrases in the work's `prose` field. At `286c5e53a` it reported 20 errata in the four verify files and 73 in
  docs/v3 (33 of them without `--prose`); all are 0 now. The run also caught Meta-Harness (a wrong title and author in 9 files), the RSI survey,
  AI4AI-Bench, PooL, the bacterial-luminescence paper and ActPlane's invented "execution planes"; those are fixed.
- Follow-up: `depth/05-agent/harness-engineering.md` builds on "six harness principles" attributed to Meta-Harness, but
  the real paper (an outer-loop search over harness code) does not describe principles in its abstract; the section
  needs a reading of the full paper.
