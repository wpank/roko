+++
id = "gap-8cf47b"
kind = "gap"
title = "Research paper Appendix E: Table E.2 says no S01 record type exists at a17d4dadd; recheck Built by at a newer pin"
status = "open"
triage = "unverified"
severity = "p3"
goal = "whitepaper"
size = "S"
subsystem = ["paper"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:47, wk-rp-appCD's report on gap-e2d092)"
anchors = ["tmp/cybernetic-harness/paper/sections/E-reproducibility.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = ["gap-528762", "gap-e2d092", "gap-2abf34"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'None exists at .a17d4dadd.' tmp/cybernetic-harness/paper/sections/E-reproducibility.md"
+++

## Problem

Appendix E's Table E.2 lists the S01 record types as designed and says "None exists at `a17d4dadd`." S01 v1.2 now maps those records to Rust types that gap-528762 merged later (`AttemptVerdictRecord`, `RunProvenanceManifest`, `GateVerdictTag` and others; merge `1c5371f9a`, an ancestor of BASE). The table's "Built by" column and row CE.2 (`MISSING@a17d4dadd (gap-528762, gap-8cb382)`) no longer describe the code.

## Why it matters

Research paper (epic spec-f8d196): Appendix E is the reproducibility appendix, and its status claims must match a named commit.

## Where

`tmp/cybernetic-harness/paper/sections/E-reproducibility.md`: Table E.2 (about :76-90) and the claims table (CE.2, about :445). S01 v1.2 §5 (`tmp/cybernetic-harness/specs/S01-instrumentation.md`) has the name mapping.

## Current state

The paper is pinned at `a17d4dadd`, which is older than `1c5371f9a`.

## Plan

1. Choose the pin for this pass: a commit that contains `1c5371f9a`, the same one the rest of the paper moves to if the whole paper is re-pinned.
2. For each Table E.2 row, fill "Built by" with the type and file at that pin, or keep "designed" where the record is still missing.
3. Re-tag CE.2, and any other row that cites gap-528762.
4. Run paperlint on the file.

## Done when

- [ ] Table E.2 and CE.2 state what exists at a named pin that includes gap-528762.
- [ ] The `[[verify]]` command passes.

## Notes

- `tmp/cybernetic-harness/` is untracked: edit in place in the main checkout.
