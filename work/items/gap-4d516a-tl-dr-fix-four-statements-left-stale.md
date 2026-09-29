+++
id = "gap-4d516a"
kind = "gap"
title = "TL;DR: fix four statements left stale after the 2026-09-29 refresh"
status = "done"
triage = "verified"
severity = "p3"
goal = "whitepaper"
size = "S"
subsystem = ["tldr"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "193ee093c"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (14:48 wk-tldr; 14:58 item e; 15:09 wk-wp-s7)"
anchors = ["tmp/cybernetic-harness/tldr/00-README.md", "tmp/cybernetic-harness/tldr/research/C1-research-planning-decomposition-cascades.md", "tmp/cybernetic-harness/tldr/research/B7-real-run-evidence.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = ["gap-b64fba", "gap-cdd5f4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f tmp/cybernetic-harness/tldr/00-README.md && ! grep -q '13 decisions are still open' tmp/cybernetic-harness/tldr/00-README.md && test -f tmp/cybernetic-harness/tldr/research/C1-research-planning-decomposition-cascades.md && ! grep -q 'First fix bug-28becc' tmp/cybernetic-harness/tldr/research/C1-research-planning-decomposition-cascades.md && test -f tmp/cybernetic-harness/tldr/research/B7-real-run-evidence.md && ! grep -q 'to +300 s' tmp/cybernetic-harness/tldr/research/B7-real-run-evidence.md && ! grep -q 'merged three times' tmp/cybernetic-harness/tldr/research/B7-real-run-evidence.md && ! grep -q '3 hand merges' tmp/cybernetic-harness/tldr/research/B7-real-run-evidence.md"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "TL;DR (tmp, wk-tldr2): 00-README item 10 now says 8 of the 13 decisions in 05 §6 are open, with a changelog naming the 5 settled on 09-29 and their sources; research/C1 implication 9 rewritten around ViabilityBench (dec-b78874; bug-28becc named only for roko bench swe); research/B7 join window -30 s..+120 s, TL;DR and table rows say 5 hand merges, 3 with semantic breaks, isolation row ORPHANED per matrix IS2; frozen copy unchanged (SHA256SUMS passes). Verify passes in MAIN. Follow-ups filed as gap-f3be74."
+++

## Problem

The tldr refresh (gap-b64fba, closed) left four statements that later findings contradict:

1. **The open-decision count.** `tldr/00-README.md:52` says "13 decisions are still open". Several of tldr/05 §6's
   decisions have been settled since, for example parking dreams (decision 12) through dec-e70592 and bug-470de8.
2. **The benchmark's first step.** `tldr/research/C1-…-cascades.md:68`, implication 9, says "First fix bug-28becc:
   `roko bench swe` leaks the gold patch". The pilot now runs on ViabilityBench in `benchmarks/viabilitybench/`
   (dec-b78874), which does not use `roko bench swe`, so bug-28becc is no longer on the benchmark's path.
3. **B7's join window.** `tldr/research/B7-real-run-evidence.md:13` says an attempt's gate row is joined within
   "−30 s to +300 s". Its figures (101 of 373 false greens) come from a forward window of at most 120 s; with +300 s
   the same code gives 102 (`companion-audit/E1-REDERIVATION.md:297-309`, `:378`).
4. **B7's merge count.** B7's TL;DR (line 38: "merged three times") and two table rows (lines 48 and 64: "3 hand
   merges") contradict its own corrections at lines 86 and 166: there were 5 hand merges, 3 of them with semantic
   breaks.

## Why it matters

The tldr is the short account the papers and the whitepaper draw on, and the brief's canonical numbers cite B7. Epic
spec-f8d196.

## Where

The three files above, in `tmp/cybernetic-harness/tldr/`.

## Current state

Checked in MAIN on 2026-09-29: all four statements are present.

## Plan

1. Recount the open decisions against `tmp/cybernetic-harness/DECISIONS.md` and `work/DECISIONS.md`, and name the
   settled ones.
2. Rewrite implication 9 around ViabilityBench. Mention bug-28becc only for `roko bench swe` itself.
3. State B7's window as −30 s to +120 s, and point to E1's reconciliation.
4. Say "5 hand merges, 3 with semantic breaks" in the TL;DR and both table rows.

## Done when

- [ ] The four statements are corrected.
- [ ] The `[[verify]]` command passes.

## Notes

- These files are untracked, so edit them in place in the main checkout.
- Don't edit the frozen copy of B7 in `docs/whitepaper/evidence/2026-09-29-b7-real-run-evidence.md`. It stays byte
  for byte, and the whitepaper cites its sha256.
