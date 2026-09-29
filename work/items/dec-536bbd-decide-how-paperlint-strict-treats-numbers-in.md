+++
id = "dec-536bbd"
kind = "decision"
title = "Decide how paperlint --strict treats numbers in the claims-ledger rows"
status = "open"
triage = "unverified"
severity = "p2"
goal = "whitepaper"
size = "S"
subsystem = ["paper", "tools/paperlint"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (15:13, next paper wave)"
anchors = ["tmp/cybernetic-harness/paper/00-README.md", "tmp/cybernetic-harness/paper/tools/claims.py"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = ["gap-af0b57", "gap-c4d630", "gap-b605cf"], supersedes = [], duplicate_of = "" }
+++

## Problem

Each research-paper section ends with a "Claims ledger" table: one row per claim, with its type and its evidence in a
column. `paper/tools/claims.py` reads these tables to regenerate `CLAIMS-EVIDENCE.md`. paperlint (`tools/paperlint.py`,
merged in `464cbabba` and `d5c1dc6be`, after this item's base) leaves ledger sections out of the word budget, but its
strict `number` rule still applies to them. That rule fails any `$` amount, percentage, N× ratio or "N of M" count
that has neither an inline `[@key]` nor a footnote naming a source. Ledger rows state numbers and name their source in
the evidence column, so `--strict` flags every ledger. The whitepaper's sections have no ledger tables (its claims
table is in `docs/whitepaper/README.md`), which is why it passes.

## Why it matters

The research paper's final lint is `--strict`. Until this is settled it cannot pass, or its ledgers get reformatted
twice. Epic spec-f8d196.

## Where

- `tools/paperlint.py` (`check_numbers`, and the ledger exemption used for word counts).
- `tmp/cybernetic-harness/paper/00-README.md` (the ledger format and marker grammar) and `paper/tools/claims.py`.

## Current state

Reported at 15:13 on 2026-09-29, after paperlint's final merge. No decision yet.

## Plan

The options:

1. **Exempt ledger sections from the `number` rule**, as they are already exempt from the budget. The evidence column
   is the source. Cheapest, and consistent with the ledger being metadata for `claims.py`.
2. **Keep the rule, and give each ledger row a footnote** naming its source. This adds a lot of footnotes, and the
   evidence column says the same thing.
3. **Move the ledgers out of the section files** into `CLAIMS-EVIDENCE.md` only. `claims.py` would need a new input
   format.

The default is option 1, with a test in `tools/test_paperlint.py`: a ledger row with a number and an evidence cell
passes `--strict`.

## Done when

- [ ] The choice is recorded here and in `paper/00-README.md`.
- [ ] paperlint implements it, with its test, and `--strict` over a section with a ledger reports no `number` finding
      from the ledger.

## Notes

- paperlint serves both papers. Keep the whitepaper's behaviour unchanged.
