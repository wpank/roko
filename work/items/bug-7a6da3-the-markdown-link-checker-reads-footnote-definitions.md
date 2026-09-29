+++
id = "bug-7a6da3"
kind = "bug"
title = "The markdown link checker reads footnote definitions as reference links"
status = "open"
triage = "unverified"
severity = "p3"
goal = "whitepaper"
size = "S"
subsystem = ["tooling/docs-integrity"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (15:13, next paper wave)"
anchors = ["tools/docs_integrity/check_markdown_links.py:81", "tools/docs_integrity/check_markdown_links.py:357", "tools/docs_integrity/test_check_markdown_links.py"]
lane = "docs"
parent = "spec-ce1484"
links = { depends_on = [], blocks = [], related = ["bug-f279ea", "gap-af0b57"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_footnote_definitions_are_not_links' tools/docs_integrity/test_check_markdown_links.py && python3 -m unittest tools.docs_integrity.test_check_markdown_links -k test_footnote_definitions_are_not_links"
+++

## Problem

`tools/docs_integrity/check_markdown_links.py` recognises a reference-link definition with
`_REFERENCE_TARGET_RE = ^ {0,3}\[[^\]\n]+\]:[ \t]*(\S+)` (`:81`). A footnote definition such as
`[^3]: Commit abc1234; see …` matches it too, so the parser takes the first word after the colon as a link target
(`:357-361`) and reports it as a missing local file.

## Why it matters

The whitepaper puts every number's source in a footnote (paperlint's `number` rule), so its sections are full of
footnote definitions. The Docs Lint workflow (`.github/workflows/docs-lint.yml`) runs this checker, and gap-af0b57's
design reuses it for local links. False failures either block docs CI on the whitepaper or teach people to ignore the
checker. Epic spec-ce1484.

## Where

- `tools/docs_integrity/check_markdown_links.py`: `_REFERENCE_TARGET_RE` (`:81`) and its use in the parser
  (`:357-361`).
- `tools/docs_integrity/test_check_markdown_links.py`: the unittest suite; run it from the repo root with
  `python3 -m unittest tools.docs_integrity.test_check_markdown_links`.

## Current state

Reproduced at `4c0326dfc`: a file containing `[^1]: Commit abc1234; see the log.` gives
`local link target does not exist: Commit`. Reported while checking the whitepaper on 2026-09-29.

## Plan

1. Skip labels that start with `^`, for example `^ {0,3}\[(?!\^)[^\]\n]+\]:`, so footnote definitions are never read
   as link targets.
2. Add `test_footnote_definitions_are_not_links`: a file with a footnote definition whose text starts with a word that
   is not a file reports no broken link, while a real reference definition to a missing file still does.

## Done when

- [ ] Footnote definitions produce no link findings, and reference definitions are still checked.
- [ ] The `[[verify]]` command passes, and the rest of the suite still passes.

## Notes

- bug-f279ea edits the same file, in its registry checks. The two changes don't overlap, but merge one after the
  other.
