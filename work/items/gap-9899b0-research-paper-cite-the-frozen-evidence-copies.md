+++
id = "gap-9899b0"
kind = "gap"
title = "Research paper: cite the frozen evidence copies in §1.2, §4.1, §7 and Appendix F"
status = "open"
triage = "unverified"
severity = "p2"
goal = "whitepaper"
size = "S"
subsystem = ["paper"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:02, wk-rp-cite's report on gap-184da5)"
anchors = ["tmp/cybernetic-harness/paper/sections/01-introduction.md", "tmp/cybernetic-harness/paper/sections/04-system.md", "tmp/cybernetic-harness/paper/sections/07-results-regulation.md", "tmp/cybernetic-harness/paper/sections/F-ai-assistance-ethics.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = ["gap-184da5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rqE 'tldr/research/(B7|B5)|assessment/W12|CASES\\.md' tmp/cybernetic-harness/paper/sections/01-introduction.md tmp/cybernetic-harness/paper/sections/07-results-regulation.md && grep -q 'docs/whitepaper/evidence/' tmp/cybernetic-harness/paper/sections/01-introduction.md"
+++

## Problem

gap-184da5 moved §8, §9 and Appendix E onto the frozen evidence copies in `docs/whitepaper/evidence/`. Other places still cite live paths for B7, W12, the field cases and B5 (wk-rp-cite, 2026-09-29):

- §1.2 `[^field]`, with claims C1.50–C1.53 (B7, W12, CASES);
- §4.1, claim C4.38 (B7);
- §7.3 `[^loops]`, claim C7.18 and §7's source line (B5);
- Appendix F.7, claims CF.21–CF.22 (W12, B7).

## Why it matters

A number must cite a frozen, hashed copy. A live file can change after the paper is written. Epic spec-f8d196.

## Where

The four section files named in the anchors (check the exact file names in `paper/sections/`), plus the frozen files and their `SHA256SUMS`.

## Current state

The live paths are still cited.

## Plan

1. Re-point each citation to its frozen copy, using the footnote form from §8 and §9 (the path plus a 12-hex sha256 prefix).
2. Update the claims-ledger rows to match.
3. Keep every `[[RESULT …]]` slot, and keep each file within 1.2× its word budget.

## Done when

- [ ] No section cites live paths for these sources.
- [ ] The `[[verify]]` command passes, and paperlint `--budget 1.2 --check-identifiers` is clean on the four files.
