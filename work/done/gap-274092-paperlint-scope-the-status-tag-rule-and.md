+++
id = "gap-274092"
kind = "gap"
title = "paperlint: scope the status-tag rule and the claims-ledger skip so the companion lints correctly"
status = "done"
triage = "verified"
severity = "p3"
goal = "whitepaper"
size = "S"
subsystem = ["paper", "tooling"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "79edaeb93"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:40, wk-companion-review's report on gap-a3031b)"
anchors = ["tools/paperlint.py", "tools/test_paperlint.py"]
lane = "docs"
parent = "spec-ce1484"
links = { depends_on = [], blocks = [], related = ["gap-af0b57", "gap-cb86e4", "gap-9cb0b9"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_claim_level_names_in_code_or_tables_are_not_status_tags' tools/test_paperlint.py && grep -qw 'def test_ledger_skip_is_limited_to_ledger_sections' tools/test_paperlint.py && python3 -m unittest tools.test_paperlint"

[closed]
at = 2026-09-29
commit = "79edaeb93"
by = "wk-paperlint2"
evidence = "79edaeb93: a bare status tag in a table or a code span is a name, a line <!-- paperlint: claim-levels WIRED PARTIAL --> declares a file's own levels, and TAG@sha is checked everywhere; the word count skips only headings 'Claims ledger' or 'Claims ledger (§…)'. The [[verify]] passes (27 tests; the 3 new ones fail on 2a9312985). E10-DRAFT.md, same content: status-tag findings 19 before, 15 after, 0 with the directive line, which the companion's owner adds (gap-cb86e4); §4.1 now counts (+285 words). paperlint --strict --require-status reviewed docs/whitepaper/*.md: 14 files clean. Research paper (--budget 1.2 --check-identifiers): every section's word count is identical to 2a9312985's."
+++

## Problem

Two paperlint rules misfire on the companion report (wk-companion-review, 2026-09-29):

- The companion's claim levels WIRED and PARTIAL share names with the whitepaper's status tags. paperlint reads them as untimed status tags: 19 false positives.
- The rule that skips "Claims ledger" sections also skips the companion's §4.1, which is prose.

## Why it matters

gap-cb86e4 can't reach strict-clean while the lint reports false positives. Epic spec-ce1484.

## Where

`tools/paperlint.py`; tests in `tools/test_paperlint.py`.

## Current state

Both rules match too broadly.

## Plan

1. Treat a status tag as one only in `TAG@sha` form or in prose. Allow claim-level names in tables and code spans, or behind an opt-out directive.
2. Limit the ledger skip to sections whose heading is exactly "Claims ledger".
3. Add the two tests the verify names.

## Done when

- [x] The companion's false positives are gone, and the whitepaper still lints clean.
- [x] The `[[verify]]` command passes.
