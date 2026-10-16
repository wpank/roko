+++
id = "bug-586260"
kind = "bug"
title = "work.py sync treats a mid-sentence \"fixed <id>'s …\" in any commit body as a closure, and closed the whitepaper epic by mistake"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["unknown"]
created = 2026-10-16
updated = 2026-10-16
source = "coordinator 2026-10-16: false sync closure of spec-ce1484 (commit b91be1473)"
anchors = ["tools/work.py"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_sync_ignores_a_fix_mention_in_prose' tools/test_work.py && python3 tools/test_work.py -k test_sync_ignores_a_fix_mention_in_prose"
+++

## Problem

`tools/work.py sync` closes any open item named after a closing keyword anywhere in any commit message
(`CLOSE_RE`: close/fix/resolve and their forms, followed by ids). The match is not limited to trailers or to the
start of a line, so prose that mentions fixing part of an item counts as a closure. Once that item's static verify
turns green for any reason, the next sync closes it and credits the unrelated commit.

On 2026-10-16 this closed the whitepaper epic spec-ce1484. Commit `b91be1473` (bug-ac2a01, retiring
`tools/status_matrix.py`) says in its body "fixed spec-ce1484's Tools line". Once gap-6a83de's review made the
epic's verify pass, a sync closed the epic as "Closed by b91be1473", although its Done-when still lists the open
publication item gap-8117a8. The coordinator caught it before it was committed and undid it.

## Why it matters

Closures are the tracker's record of what is finished. A false closure hides open work, it credits the wrong commit
and executor ("unknown"), and every later sync repeats it, because the old commit stays in the log. Workers write
"fixed X's …" in commit bodies routinely.

## Where

- `tools/work.py`: `CLOSE_RE` (line 74) and `sync_candidates` (around line 1608), which scan subject plus body.
- `tools/test_work.py`.
- `work/README.md`, which documents the trailers.

## Plan

1. Count only a `Closes:` trailer line, or a closing keyword at the start of the subject (`fix: …` is a type prefix,
   not an id, so subjects like `close gap-x, …` still count).
2. Keep `touched` and `drift` reporting mentions as they do now.
3. Add the test below, built from the 2026-10-16 wording.

## Done when

- [ ] A commit whose body says "fixed <id>'s Tools line" does not close `<id>` on sync, while `Closes: <id>` does.
- [ ] The test below passes.

## Notes

- Found 2026-10-16 by the coordinator after the bug-ac2a01 worker flagged its own wording as a likely false match.
