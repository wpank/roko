+++
id = "gap-8d2c79"
kind = "gap"
title = "Whitepaper review: an independent read and a strict paperlint pass"
status = "done"
triage = "verified"
severity = "p1"
goal = "whitepaper"
size = "S"
subsystem = ["docs/whitepaper"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "e84bd3462"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e1"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W9-paper-workstream.md (risk 2; rules for keeping ideal and as-built honest)"
anchors = ["docs/whitepaper/REVIEW.md", "docs/whitepaper"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = ["gap-353d57", "gap-370d3c", "gap-4161ea", "gap-ac4646", "gap-e8cb4d", "gap-424bf8", "gap-29a64e", "gap-2aad7d", "gap-c19902", "gap-ec516e", "gap-d1d92c", "gap-af0b57"], blocks = [], related = ["gap-af0b57", "gap-35a614"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f docs/whitepaper/REVIEW.md && grep -q '^Verdict: accept' docs/whitepaper/REVIEW.md && test -f tools/paperlint.py && python3 tools/paperlint.py --strict --require-status reviewed docs/whitepaper"

[closed]
at = 2026-09-29
commit = "f5a6ac64b"
by = "wk-wp-review (claude-agent)"
evidence = "Review recorded in docs/whitepaper/REVIEW.md: 28 findings with dispositions, ending 'Verdict: accept'. Fixes: f173fa9e1, 97c0c22e1, 7df1354b5, d8cd83b23, 856e52cac, 5c144601f, c2f4772e8. README publication-clean: fbfdedb5b, 1852c5d5b. Matrix, with the coordinator's approval: 45829814d (appendix status reviewed); eee9c3dd5 (next items for EX8, SS3 and DM2). SS6 and IS6 are unchanged, because the tool checks items at the pin. Synced with the coordinator branch at 80fa2171b (d64ba4434). The [[verify]] passes at f5a6ac64b: paperlint --strict --require-status reviewed docs/whitepaper reports 12 files clean, and the *.md glob reports 14 clean. status_matrix.py --check and the evidence SHA256SUMS pass. §0–§10 total 7,135 words (1.10× of 6,500)."
+++

## Problem

Each section's writer checks only that section, so nobody checks the whole. Before publishing, one reader who wrote
none of it must read the draft against its sources, and the whole directory must pass the strict lint.

## Why it matters

paperlint catches problems of form: unsourced numbers, phantom identifiers, untagged status. It cannot tell:
- whether a source says what the text claims;
- whether §1's promises match §8's plan;
- whether the tags match the matrix.

W9 (risk 2) warns that, without enforcement, ideal-state writing repeats docs/v3's claims.

## Where

- **New `docs/whitepaper/REVIEW.md`:** the reviewer, the commit, the findings, their dispositions and the verdict.
- **The section files:** the fixes, and each file's move to `Status: reviewed`.

## Current state

Checked at `41c7ffbd6`: there is nothing to review yet. This item waits for the eleven content items.

## Plan

1. **The reader:** the author, or a Claude session that drafted no section and is given no drafting context.
2. **The checks,** all at one commit:
   - every number against its footnote source;
   - every tag against the status matrix (refresh the matrix first, gap-35a614);
   - every identifier and path at HEAD;
   - the claims in the abstract and §1 against §7–§9;
   - no effect claimed without data;
   - the style rules.
3. **The record:** list each finding in `REVIEW.md` with its disposition, either "fixed in commit X" or "accepted,
   because…". End the file with `Verdict: accept` once no finding is open.
4. **The final pass:**
   - refresh the abstract's numbers;
   - set every section to `Status: reviewed`;
   - run `paperlint --strict --require-status reviewed docs/whitepaper`.

## Done when

- [ ] `REVIEW.md` lists every finding with a disposition, and ends with `Verdict: accept`.
- [ ] The `[[verify]]` command passes.

## Notes

- **Whether `REVIEW.md` goes into the PDF** is gap-8117a8's call. It stays in the repo as the review record.
- Lane `paper`; no hot files.
- **Reviewed on `work/gap-8d2c79`** (2026-09-29, wk-wp-review): `REVIEW.md` lists 25 findings with dispositions and
  ends with `Verdict: accept`; §0–§10 say `Status: reviewed`; `paperlint --strict docs/whitepaper` passes. The
  verify's one remaining failure was the appendix header, which `data/mechanisms.toml` rendered as `draft`. With the
  coordinator's approval, `45829814d` set `[matrix] status` to `reviewed` and regenerated the appendix; the verify
  then passed.
- **Follow-ups on the same branch, after the first close at `e84bd3462`:**
  - the README became publication-clean;
  - the matrix's next-item ids were fixed;
  - the coordinator branch was synced.

  The `[closed]` block now points at `f5a6ac64b`, where the verify still passes.
