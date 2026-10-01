+++
id = "bug-1440cd"
kind = "bug"
title = "tools/work.py's static_prefix treats a verify part as static unless it starts with a heavy command, so a cargo loop counts as static"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["tools/work.py"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "878d6ecd3"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (the coordinator's report)"
anchors = ["tools/work.py"]
lane = "tracker"
links = { depends_on = [], blocks = [], related = ["bug-779ae7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c 'import sys; sys.path.insert(0, \"tools\"); import work; cmd = \"for i in $(seq 1 20); do cargo test -p roko-cli --lib x; done\"; sys.exit(0 if work.static_prefix(cmd) == \"\" else 1)'"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T08:16:23Z"
commit = "878d6ecd3"
by = "wk-gates"
executor = "claude-agent"
via = "work-batch"
model = "claude-opus-5-5"
forced = false
evidence = "878d6ecd3: static_prefix breaks at any part is_heavy finds a heavy command in: at its start, after do/then/;/|/(/$(/backtick/{ or VAR= assignments, inside sh -c '…' and double-quoted command substitutions; quoted grep patterns don't count. Of today's 774 verify commands, 7 static prefixes change, each one ran cargo or gh (bug-779ae7's loop, gap-d65a17 and gap-85f102 subshells, bug-7257cb's backticks, three \"$(gh api …)\" checks). tools/test_work.py test_static_prefix_stops_at_a_heavy_command_anywhere_in_a_part; the [[verify]] command passes."
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
