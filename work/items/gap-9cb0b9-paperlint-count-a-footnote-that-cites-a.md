+++
id = "gap-9cb0b9"
kind = "gap"
title = "paperlint: count a footnote that cites a frozen evidence file by sha256 as a source"
status = "done"
triage = "verified"
severity = "p3"
goal = "whitepaper"
size = "S"
subsystem = ["paper", "tooling"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "81ade62fe"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (15:49, wk-wp-review's report on gap-8d2c79)"
anchors = ["tools/paperlint.py", "tools/test_paperlint.py"]
lane = "docs"
parent = "spec-ce1484"
links = { depends_on = [], blocks = [], related = ["gap-af0b57", "gap-8d2c79"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_sha256_footnote_counts_as_source' tools/test_paperlint.py && python3 -m unittest tools.test_paperlint -k test_sha256_footnote_counts_as_source"

[closed]
at = 2026-09-29
commit = "81ade62fe"
by = "wk-paperlint2"
evidence = "Premise stale at BASE: 0cca5d0cb (15:11) already counts a footnote naming an existing evidence/ file; the two footnotes naming gap-29a64e came from f173fa9e1, a branch without that rule. 81ade62fe adds test_sha256_footnote_counts_as_source, which pins the README's form (the first 12 hex digits, a digest on the next line, the whole digest), and plan step 2: a sha256 written right after an evidence/ path must start the file's line in the SHA256SUMS beside it, or the footnote gets a number finding (test_sha256_footnote_must_match_sha256sums, which fails on 2a9312985). The README sentence that said such a footnote does not count now states the rule. The [[verify]] passes (29 tests). All 20 digests the whitepaper cites match SHA256SUMS; paperlint --strict --require-status reviewed docs/whitepaper/*.md: 14 files clean."
+++

## Problem

paperlint doesn't count a footnote that cites only a frozen evidence file and its sha256 as a source for a number. Two whitepaper footnotes had to name gap-29a64e as well, just to satisfy the check (wk-wp-review, 2026-09-29).

## Why it matters

Frozen evidence cited by sha256 is the whitepaper's preferred source form (`docs/whitepaper/README.md`). Epic spec-ce1484.

## Where

The source rules in `tools/paperlint.py`; tests in `tools/test_paperlint.py`.

## Current state

A footnote counts as a source only if it names a commit, snapshot, rollup key, `[@key]` or work item.

## Plan

1. Accept a footnote that names a file under `docs/whitepaper/evidence/` together with a 64-hex sha256.
2. Optionally check the hash against `SHA256SUMS`.
3. Add `test_sha256_footnote_counts_as_source`.

## Done when

- [x] Such a footnote counts as a source.
- [x] The `[[verify]]` command passes.
