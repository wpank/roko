+++
id = "gap-cb86e4"
kind = "gap"
title = "Companion report: make the draft pass paperlint --strict apart from the rater and author markers"
status = "open"
triage = "unverified"
severity = "p2"
goal = "whitepaper"
size = "M"
subsystem = ["companion"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (15:47, wk-companion-fin's report on gap-b409fa)"
anchors = ["tmp/cybernetic-harness/companion-audit/E10-DRAFT.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = ["gap-a3031b"], blocks = [], related = ["gap-b409fa", "gap-a3031b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test $(python3 tools/paperlint.py --strict tmp/cybernetic-harness/companion-audit/E10-DRAFT.md | grep -E ': \\[' | grep -vE 'leftover marker \\[\\[(E2|E3|E4|E6|E11|AUTHOR)' | wc -l) -eq 0"
+++

## Problem

`python3 tools/paperlint.py --strict` on `E10-DRAFT.md` reports 114 problems (wk-companion-fin, 2026-09-29):

- 34 open markers;
- 55 numbers without a per-paragraph source footnote;
- 15 claim levels read as status tags;
- 9 identifiers;
- 1 header.

The E2, E3, E4, E6, E11 and AUTHOR markers wait for human raters and Will.

## Why it matters

The companion report is the first publication candidate, and it needs the same source discipline as the whitepaper. Epic spec-f8d196.

## Where

`tmp/cybernetic-harness/companion-audit/E10-DRAFT.md`, after the E12 review (gap-a3031b).

## Current state

114 problems.

## Plan

1. **Header:** add the status header.
2. **Numbers:** give every number a source footnote naming its commit, snapshot, rollup key or `[@key]`. The re-derivation at `91b4745f8` is the source of most of them.
3. **Claim levels:** reword them so they don't read as status tags, or tag them from the matrix.
4. **Identifiers:** tag unbuilt identifiers with `[[AS-BUILT: …]]` or "(designed)".
5. **Markers:** leave the rater and author markers in place.

## Done when

- [ ] Only the rater and author markers remain.
- [ ] The `[[verify]]` command passes.
