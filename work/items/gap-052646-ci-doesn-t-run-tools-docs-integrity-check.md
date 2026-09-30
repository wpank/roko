+++
id = "gap-052646"
kind = "gap"
title = "CI doesn't run tools/docs_integrity/check_citation_errata.py"
status = "open"
triage = "unverified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["ci/workflows", "tools/docs_integrity"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-rp-cite's report)"
anchors = [".github/workflows/docs-lint.yml", "tools/docs_integrity/check_citation_errata.py"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["gap-b23ebd"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'check_citation_errata' .github/workflows/docs-lint.yml"
+++

## Problem

gap-b23ebd built `tools/docs_integrity/check_citation_errata.py`, which checks docs/v3 against the adjudicated citation errata (`citation_errata.json`), and fixed the 649 errors it found. `.github/workflows/docs-lint.yml` doesn't run it, so new citation errors can land unnoticed.

## Why it matters

Release: docs/v3 is about to be public. Without CI, the fix erodes. p3.

## Where

`docs-lint.yml`.

## Plan

1. Add a step that runs `check_citation_errata.py` (and its tests) on changes under `docs/`.

## Done when

- [ ] The docs lint workflow runs the checker.
- [ ] The `[[verify]]` command passes.

## Notes

- CI belongs to another stream. Coordinate the edit to `docs-lint.yml` with it.
