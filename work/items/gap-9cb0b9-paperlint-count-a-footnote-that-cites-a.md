+++
id = "gap-9cb0b9"
kind = "gap"
title = "paperlint: count a footnote that cites a frozen evidence file by sha256 as a source"
status = "open"
triage = "unverified"
severity = "p3"
goal = "whitepaper"
size = "S"
subsystem = ["paper", "tooling"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (15:49, wk-wp-review's report on gap-8d2c79)"
anchors = ["tools/paperlint.py", "tools/test_paperlint.py"]
lane = "docs"
parent = "spec-ce1484"
links = { depends_on = [], blocks = [], related = ["gap-af0b57", "gap-8d2c79"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_sha256_footnote_counts_as_source' tools/test_paperlint.py && python3 -m unittest tools.test_paperlint -k test_sha256_footnote_counts_as_source"
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

- [ ] Such a footnote counts as a source.
- [ ] The `[[verify]]` command passes.
