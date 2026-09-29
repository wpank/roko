+++
id = "gap-184da5"
kind = "gap"
title = "Research paper: cite the frozen rollup, B7, W12 and the field cases from docs/whitepaper/evidence/"
status = "done"
triage = "verified"
severity = "p2"
goal = "whitepaper"
size = "S"
subsystem = ["paper"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "80fa2171b"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (14:51, wk-rp-s89's report on gap-04b0ff)"
anchors = ["tmp/cybernetic-harness/paper/sections/08-discussion.md", "tmp/cybernetic-harness/paper/sections/09-limitations.md", "tmp/cybernetic-harness/paper/sections/E-reproducibility.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = ["gap-29a64e"], blocks = [], related = ["gap-04b0ff", "gap-2abf34", "bug-469537"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f tmp/cybernetic-harness/paper/sections/08-discussion.md && ! grep -q 'TODO: cite the frozen rollup' tmp/cybernetic-harness/paper/sections/08-discussion.md && grep -q 'docs/whitepaper/evidence/2026-09-29-field-rollup' tmp/cybernetic-harness/paper/sections/E-reproducibility.md && grep -q 'docs/whitepaper/evidence/' tmp/cybernetic-harness/paper/sections/09-limitations.md"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "Research paper §8.4, §9.3 and App E (tmp, wk-rp-cite) cite the frozen copies in docs/whitepaper/evidence/ via footnotes with sha256 prefixes; ledger rows C8.28-C8.32, C9.29-C9.31, CE.20-CE.22 re-pointed and CE.25 added; new Table E.7 lists the 7 frozen files with full digests (shasum -c passes; all 38 hash citations checked against SHA256SUMS). Verify passes; paperlint --budget 1.2 --check-identifiers clean (§9 1.13x); CLAIMS-EVIDENCE regenerated (418 claims, --check up to date). Remaining live-path citations filed as a follow-up."
+++

## Problem

The research paper's §8, §9 and Appendix E quote field numbers from live, untracked files:

- the rollup (`evidence/field/ROLLUP.md`), which every capture pass regenerates;
- research note B7 (the portal build and the false-green counts);
- assessment W12 (the cost of the supervising sessions);
- the field cases CASE-001 to CASE-008 (`evidence/field/CASES.md`).

§8 still carries `[[TODO: cite the frozen rollup, docs/whitepaper/evidence/ROLLUP.md …; freeze B7 …]]`
(`08-discussion.md:78`). §9 cites `evidence/field/README.md`, `research/B7` and W12 directly
(`09-limitations.md:130-132`). Appendix E cites `ROLLUP.md` and W12 (`E-reproducibility.md:346-420`) and nothing
frozen.

## Why it matters

A number that cites a file which changes on every capture cannot be reproduced. The whitepaper has already frozen these
inputs with their sha256 sums (gap-29a64e, merged in `f5b070d86` and `1322a3cbe`):
`docs/whitepaper/evidence/2026-09-29-field-rollup.{md,json}`, `2026-09-29-b7-real-run-evidence.md`,
`2026-09-29-w12-operator-loop-cost.md`, `2026-09-29-field-cases.md`, `2026-09-29-field-readme.md`, `SHA256SUMS`. The
research paper should cite the same frozen copies, so both papers quote one set of numbers. Epic spec-f8d196.

## Where

The three section files above. The frozen inputs are in `docs/whitepaper/evidence/`, which does not exist at
`4c0326dfc` and arrives with gap-29a64e's merge.

## Current state

Checked in MAIN on 2026-09-29: §8 has the TODO, and §9 and Appendix E cite the live files.

## Plan

1. Replace each citation of the live rollup, B7, W12 and CASES with the frozen file and its sha256 from `SHA256SUMS`.
2. Where the paper's number differs from the frozen copy (for example the autonomy index, now 2/41), use the frozen
   value, or state the newer window explicitly.
3. In Appendix E, list the frozen inputs as the field-evidence part of the reproducibility package.

## Done when

- [ ] §8, §9 and Appendix E cite only frozen field evidence, each with its hash.
- [ ] The `[[verify]]` command passes.

## Notes

- Untracked files: edit them in place in the main checkout.
- Keep every `[[RESULT …]]` slot. Don't edit the frozen copies.
