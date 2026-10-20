+++
id = "gap-6ee3c5"
kind = "gap"
title = "tools/work.py cannot mark a done item's verify as superseded, so every refactor grows the list of done items whose verify fails (60 at 7b6a5af44)"
status = "done"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "M"
subsystem = ["tools/work"]
created = 2026-10-15
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "f45d568fc"
source = "status-quo-2026-10-15#finding-35"
anchors = ["tools/work.py::cmd_verify", "tools/work.py::static_prefix", "tools/test_work.py", "work/README.md"]
lane = "tracker"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_superseded_verify_is_reported_apart' tools/test_work.py && python3 tools/test_work.py -k test_superseded_verify_is_reported_apart"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T15:04:53Z"
commit = "f45d568fc"
by = "work-batch worker (claude-agent)"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-16T08:07:05Z"
model = "claude-opus-5-5"
forced = false
evidence = "42da5241e: tools/work.py retire-verify <id> --index N --reason ... [--by REV|ID] [--static-only] adds superseded = '<date>: <reason>' and superseded_by (a commit or item id, default HEAD) to a done item's [[verify]] table, changing nothing else but updated, and refuses unless the item is done and that verify fails now. verify prints RETIRED <id> for such an entry, neither runs nor counts it, and lists apart the items whose every verify is retired; an open item's verify runs as before. check requires both fields, well formed, on a closed item; show numbers the entries and show --json adds verify_retired. work/README.md documents it (Retired verify commands, and the regression rule). tools/test_work.py TestRetiredVerify (test_superseded_verify_is_reported_apart, the refusals, the check rules): 42 tests OK, and the item's verify passes. The closing commit (commit field) retires 51 verifies on 46 of the 2026-10-15 assessment's 60 done items with B's reasons, through retire-verify: the whitepaper items by 7898493f2, the research-paper items by 6f85e5efc, gap-8d2c79 by gap-6a83de, gap-7ee870 by gap-aed6f1, gap-652d05 by gap-b409fa, the three companion items by gap-b90650 (decision 4), and the code items by c20dc5f5c, 0debf4769, d3f73bb45, 7f2776fc1, 42fbf09f2, 685c53311, 348cb6c3f and 39e72deef. verify --static-only over the 60, and the static sweep of done items closed since 2026-09-26, list no FAIL for any of them (51 RETIRED lines, 44 items listed apart). Still failing among the 60: B's MOVED and VERIFY-WRONG verifies (bug-f5f4d2), gap-c4d630 (reg-3621b1) and bug-d1dea8 (reg-48b7ba). Also filed bug-2f0b39 and bug-ec244b. check: 0 problems."
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
