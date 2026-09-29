+++
id = "gap-424bf8"
kind = "gap"
title = "Whitepaper §6 Measured trust"
status = "open"
triage = "verified"
severity = "p1"
goal = "whitepaper"
size = "S"
subsystem = ["docs/whitepaper"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e1"
discovered_from = "tmp/cybernetic-harness/tldr/research/C3-competitive-landscape.md (what no product documents)"
anchors = ["docs/whitepaper/06-measured-trust.md"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = ["gap-0191eb", "gap-af0b57"], blocks = [], related = ["gap-e8cb4d", "gap-2aad7d", "spec-6ac537"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f docs/whitepaper/06-measured-trust.md && grep -qi 'false-green rate' docs/whitepaper/06-measured-trust.md && test -f tools/paperlint.py && python3 tools/paperlint.py --strict docs/whitepaper/06-measured-trust.md"
+++

## Problem

tldr/00 (point 9) calls measured trust Roko's defensible ground. It has three parts:
- routing across vendors, learned from your own verified outcomes;
- a false-green rate measured by random audits;
- per-loop evidence that learning helps.

The whitepaper needs a section that defines these three measures, says how Roko is designed to produce them, and
admits that all three are missing today.

## Why it matters

These measures are what set Roko apart from Claude Code, Codex, Cursor, Kiro and Devin, which already pair frontier
planners with cheaper executors (C3). They are also the claim that is easiest to overstate.

## Where

`docs/whitepaper/06-measured-trust.md` (new; gap-0191eb creates it as a stub).

## Current state

Checked at `41c7ffbd6`.
- **Competitors:** C3 read vendor docs fetched on 2026-09-29 and found no product that documents any of the three
  measures.
- **Roko:** tldr/01 tags selling points 3, 5 and 6 MISSING. The router is PARTIAL: its LinUCB stage learns only from
  successes.
- **What exists today:** per-task verdicts have been honest since 09-28, with 0 false greens in 151 passes (research
  note B7).

## Plan

1. **Define each measure:** what it counts, its denominator, and how it is reported. The false-green rate comes with a
   confidence interval; the loop measure reports exposure, influence and benefit per loop.
2. **Explain how Roko is designed to produce each measure, with tags:**
   - M3 routing on verified labels;
   - M4 audits with hidden tests;
   - M2 loop audits.

   Point to §5 for the mechanisms and to §8 for the evaluation.
3. **Describe the field with a date:** "as documented on 2026-09-29, we found no product that…", citing the vendor
   pages. Never write "Roko is the first".
4. **State what exists now,** with its definition and source.

## Done when

- [ ] Each measure is defined, with its denominator, and tagged.
- [ ] The `[[verify]]` command passes.

## Notes

- **Needs `tools/paperlint.py`** (gap-af0b57) to close.
- **Vendor pages** need `@online` bib entries with the access date. Vendor numbers stay vendor claims (C3).
- Lane `paper`; no hot files.
- **Written 2026-09-29** on `work/gap-424bf8` at `c9e5f12de`, with five `@online` vendor entries in `167809ce0`.
  `paperlint --strict --check-identifiers` is clean against gap-af0b57's uncommitted working copy (sha256
  `91eb8dbe311c`): 553 words counting the footnote, 518 without. The verify fails on this branch only because
  `tools/paperlint.py` has not merged; close once it has. Tags match the status matrix at `a17d4dadd` (rows RC2,
  RG5, QA5, RG4). When gap-29a64e freezes CASE-001 into `evidence/`, point the `[^6-verdicts]` footnote at it.
