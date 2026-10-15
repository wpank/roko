+++
id = "gap-8ff818"
kind = "gap"
title = "paperlint --strict flags every number in a Claims ledger table: exempt ledger sections from the number rule (dec-536bbd option 1)"
status = "open"
triage = "verified"
severity = "p2"
goal = "whitepaper"
size = "S"
subsystem = ["tools/paperlint"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#decision-3"
anchors = ["tools/paperlint.py::check_numbers", "tools/paperlint.py::LEDGER", "tools/test_paperlint.py::test_ledger_skip_is_limited_to_ledger_sections"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["dec-536bbd", "gap-ac1aed", "spec-f8d196"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_strict_number_rule_skips_the_claims_ledger' tools/test_paperlint.py && python3 -m unittest tools.test_paperlint -k test_strict_number_rule_skips_the_claims_ledger"
+++

## Problem

`paperlint --strict` fails every research-paper section that ends with a "Claims ledger" table. paperlint already
leaves ledger sections out of the word budget (`tools/paperlint.py`, the `LEDGER` heading pattern at :64 and the skip at
:364), but `check_numbers` (:828) walks every block, so each ledger row's `$` amount, percentage, N× ratio or "N of M"
count is flagged as "has no source", though the row's evidence column names its source. The whitepaper passes only
because its claims table lives in `docs/whitepaper/README.md`.

## Why it matters

Will's decision 3 (2026-10-15) accepted dec-536bbd option 1: exempt claims-ledger sections from the number rule, with a
test in `tools/test_paperlint.py`. The research paper's final lint is `--strict` (decision 4: the independent review of
both papers ends with a final lint pass, gap-ac1aed), so it cannot pass until this lands.

## Where

- `tools/paperlint.py::check_numbers`; the ledger-section skip used for the word count (`LEDGER`, :364).
- `tools/test_paperlint.py` (existing ledger tests `test_word_count_leaves_out_the_claims_ledger`,
  `test_ledger_skip_is_limited_to_ledger_sections`).
- `tmp/cybernetic-harness/paper/00-README.md`: records the ledger format; note the exemption there.

## Current state

No exemption; dec-536bbd is answered as of 2026-10-15.

## Plan

1. Have `check_numbers` skip blocks inside a section whose heading fully matches `LEDGER` (only ledgers: the editorial
   "Placeholder map" and "Alternative …" sections keep the rule).
2. Add `test_strict_number_rule_skips_the_claims_ledger`: a section with a ledger row holding a percentage and an
   evidence cell passes `--strict`; the same number in a prose paragraph above it still fails.
3. One line in `paper/00-README.md` and in paperlint's usage text saying ledgers are exempt from `number`.

## Done when

The `[[verify]]` command passes, and the existing paperlint tests still pass.

## Notes

The paper hold (roko-55, decision 4's review) covers the paper text, not `tools/paperlint.py`.
