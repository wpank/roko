+++
id = "spec-6afe5c"
kind = "spec"
title = "Backlog Track W — the papers (alongside, after their inputs)"
status = "superseded"
triage = "verified"
severity = "p2"
goal = "whitepaper"
subsystem = ["unknown"]
created = 2026-10-02
updated = 2026-10-27
last_verified = 2026-10-27
last_verified_rev = "15335e2f9"
source = "tmp/backlog/2026-10-02-complete-and-wire/PACKAGES.md"
anchors = []
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-10-27
by = "work sweep 2026-10-27 (static check)"
evidence = "6 of 7 packages are in work/closed/ (wontfix or superseded), all closed 2026-10-15: PK95=gap-d4a1c1 (superseded), PK97=gap-d2507f (wontfix), PK98=gap-b90650 (wontfix), PK99=gap-9b5582 (superseded), PK100=gap-6aaee9 (superseded), PK101=gap-ac1aed (superseded). PK95's own evidence: \"closed on Will's 'mark everything done' (2026-10-15) ... The rewrite hold has run its course: session roko-55 finished both papers on 10-02 and 10-08\". Only PK96 (gap-a6dab7, figure/table scripts) is in work/done/ as true done work."
+++

## Problem

Track W: the papers (alongside, after their inputs): 7 work packages from the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (its `README.md`, `00-INDEX.md` and `PACKAGES.md`).

## Why it matters

The backlog's order is safe runs, then honest measurement, then the golden-path proof, then the cybernetic loops (S02, M2, M3, M4, M1), then domains, cleanup and deploy, with the papers alongside.

## Where

- PK95: Papers: Freeze the audit's live-run findings (R3, R4) as whitepaper evidence (+10 more)
- PK96: Papers: Figure and table scripts for the P1 results (F2–F5, F9; T3, T7, T9, T11), dry-run on… (+5 more)
- PK97: Papers: Companion: fill the E2 and E3 results from the human ratings, and redraw Figure 3 (+2 more)
- PK98: Papers: Post the companion report to arXiv and record its id (+1 more)
- PK99: Papers: Research paper §5 and Appendix D: cite the pre-registration lock for every number and… (+1 more)
- PK100: Papers: Research paper §6.4: report H3, the spec × model interaction (+9 more)
- PK101: Papers: Research paper Appendix E and the ViabilityBench README: the artefact as released (+5 more)

## Current state

Filed 2026-10-02 by the coordinator.

## Plan

Work the packages through `/work-batch`, following `BUILD-RULES.md`.

## Done when

Every package of this phase is done.

## Notes

- Held tasks and deferred decisions are not filed; see the backlog folder.

Checked 2026-10-27: 2026-10-26 sweep: the original premise (write the papers package-by-package against the empirical-draft outline) no longer applies: the 2026-10-02 design-paper rewrite (session roko-55, landed 10-02 and 10-08) replaced the target documents outright, and Will's 2026-10-15 'mark everything done' pass closed 6 of the 7 packages as wontfix/superseded for that reason. Only PK96 (figure/table scripts) is genuinely done work; the rest of this tracker's premise is gone, not satisfied.
