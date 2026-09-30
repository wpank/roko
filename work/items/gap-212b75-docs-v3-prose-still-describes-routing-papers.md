+++
id = "gap-212b75"
kind = "gap"
title = "docs/v3 prose still describes routing papers (BEST-Route, xRouter, Router-R1) under invented titles, and the CLEAR-framework sections cite a fabricated work"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
size = "M"
subsystem = ["docs/v3"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-rp-cite's report)"
anchors = ["docs/v3/20-GATEWAY.md", "docs/v3/depth/20-gateway/06-routing-research.md", "docs/v3/depth/06-composition/distributed-context-engineering.md", "docs/v3/depth/06-composition/token-budget-management.md", "tools/docs_integrity/citation_errata.json"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["gap-b23ebd", "gap-052646", "gap-fc5d3d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 tools/docs_integrity/check_citation_errata.py --prose docs/v3/20-GATEWAY.md docs/v3/depth/20-gateway/06-routing-research.md docs/v3/depth/06-composition/distributed-context-engineering.md docs/v3/depth/06-composition/token-budget-management.md"
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
