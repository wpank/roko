+++
id = "bug-1440cd"
kind = "bug"
title = "tools/work.py's static_prefix treats a verify part as static unless it starts with a heavy command, so a cargo loop counts as static"
status = "open"
triage = "unverified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["tools/work.py"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (the coordinator's report)"
anchors = ["tools/work.py"]
lane = "tracker"
links = { depends_on = [], blocks = [], related = ["bug-779ae7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c 'import sys; sys.path.insert(0, \"tools\"); import work; cmd = \"for i in $(seq 1 20); do cargo test -p roko-cli --lib x; done\"; sys.exit(0 if work.static_prefix(cmd) == \"\" else 1)'"
+++

## Problem

`static_prefix` (`tools/work.py:192`) collects the parts of a verify command up to the first one that starts with a heavy command (`HEAVY.match(part)`, :196). A part that contains a heavy command, but doesn't start with one, counts as static. bug-779ae7's verify, `for i in $(seq 1 20); do cargo test …; done`, is therefore all static. `close` runs the static prefix in the main checkout with a 60 s timeout, so it starts cargo in the main checkout's target dir, times out, and refuses the item as red.

## Why it matters

The tracker runs heavy commands where it promises cheap ones, and blocks closures that are fine. p3.

## Where

`static_prefix` in `tools/work.py`.

## Plan

1. Treat any part that contains a heavy command (`HEAVY.search`) as heavy.
2. Add a unit test with a loop verify.

## Done when

- [ ] A loop containing cargo is never part of the static prefix.
- [ ] The `[[verify]]` command passes.

## Notes

- `tools/work.py` has uncommitted edits from roko-90 in the main checkout. Coordinate with that session before editing it.
