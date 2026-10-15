+++
id = "gap-6ee3c5"
kind = "gap"
title = "tools/work.py cannot mark a done item's verify as superseded, so every refactor grows the list of done items whose verify fails (60 at 7b6a5af44)"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "M"
subsystem = ["tools/work"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-35"
anchors = ["tools/work.py::cmd_verify", "tools/work.py::static_prefix", "tools/test_work.py", "work/README.md"]
lane = "tracker"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_superseded_verify_is_reported_apart' tools/test_work.py && python3 tools/test_work.py -k test_superseded_verify_is_reported_apart"
+++

## Problem

A done item's `[[verify]]` records what proved the fix at the time. When later work deletes or renames what it
checks (the PRD pipeline's removal, the papers' rewrite as design papers, a renamed test), the verify fails although
nothing regressed. `tools/work.py` has no way to say so: `work.py verify` (`cmd_verify`) reports FAIL, and the
README's rule ("A done item whose verify command later fails is reopened as kind = regression") would reopen it. At
7b6a5af44, 60 done items fail their static verify; 35 are paper-lane items whose files the rewrites deleted, and
most of the rest are superseded by the PRD removal (bug-017c2d, bug-255765, bug-8b1bf8, bug-a5cd6b, bug-db607b and
others). Telling a real regression (two this round: bug-d1dea8, bug-8417d9) from a stale check takes a person
re-reading each one. Status-quo assessment 2026-10-15, finding 35
(`tmp/status-quo/2026-10-15/backlog/B-top-level-and-regressions.md` part 2).

## Why it matters

Goal `tooling`: the work graph is only trusted if a failing verify means something. The regression sweep
(`/work-sweep`, `work.py verify --closed-since`) drowns in false alarms today.

## Where

- `tools/work.py`: `cmd_verify` (about line 1658), the `check`/`drift` paths that read `[[verify]]`, and `close`.
- `tools/test_work.py`: the tests.
- `work/README.md`: "Verify commands" and "Status and closure rules".

## Current state

`[[verify]]` entries have only `command`. Closure evidence lives in `[closed]`.

## Plan

1. Let a `[[verify]]` entry carry `superseded = "<date>: <reason>"` and `superseded_by = "<commit or item id>"`.
2. Add `work.py retire-verify <id> --index N --reason "…" [--by <sha|id>]`, which fills those fields and refuses
   unless the item is done and the verify fails now.
3. `work.py verify` prints `RETIRED <id>` for such entries, does not run them and does not count them as failures;
   `check` requires both fields on a retired entry; a done item with every verify retired is listed apart.
4. Document it in work/README.md, then retire the superseded verifies of the 60 (B's table gives the reason for each).
5. Test `test_superseded_verify_is_reported_apart`: a done item with one retired and one passing verify passes
   `verify`; one with a failing, un-retired verify still fails.

## Done when

The `[[verify]]` command passes, `work.py check` is clean, and `work.py verify --closed-since 2026-09-26` lists no
FAIL for a verify B's report classed as superseded.

## Notes

The tracker lane is one agent at a time (`tools/work.py` is one file). Don't change how open items' verifies run.
