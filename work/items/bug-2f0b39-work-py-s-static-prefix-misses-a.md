+++
id = "bug-2f0b39"
kind = "bug"
title = "work.py's static_prefix misses a cargo step behind env -i, so verify --static-only ran dec-01be49's cargo test"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["tools/work"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "gap-6ee3c5 worker, 2026-10-16"
discovered_from = "gap-6ee3c5: verify --closed-since 2026-09-26 --static-only started a roko-cli build"
anchors = ["tools/work.py::is_heavy", "tools/work.py::static_prefix", "tools/test_work.py"]
lane = "tracker"
links = { depends_on = [], blocks = [], related = ["dec-01be49", "gap-de8fdc", "gap-6ee3c5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c \"import sys; sys.path.insert(0, 'tools'); import work; sys.exit(work.static_prefix('env -i HOME=x cargo test -p c t') != '')\""
+++

## Problem

`python3 tools/work.py verify --closed-since 2026-09-26 --static-only` runs `cargo test -p roko-cli --lib doctor::tests::`.
dec-01be49's verify is `env -i HOME="$(mktemp -d)" CARGO_HOME=… PATH=… cargo test -p roko-cli --lib doctor::tests::`,
and `is_heavy` does not see its cargo step, so `static_prefix` returns the whole command as "static".
`HEAVY_ANYWHERE` accepts `env` followed only by `VAR=value` assignments; an option before them (`-i`, `-u NAME`) breaks
the match. Seen on 2026-10-16 while running the static regression sweep for gap-6ee3c5: a worker told not to run
cargo got a cold roko-cli build (7 GB of `target/` in its worktree) from a `--static-only` run.

## Why it matters

Goal `tooling`: `--static-only` promises never to run a cargo, npm or network step, and `close`, `sync` and
`retire-verify --static-only` judge an item by the same static prefix. Any verify written as `env -i … cargo …` (the
pattern for running a test without the caller's environment) silently starts a build there.

## Where

`tools/work.py`: `HEAVY_ANYWHERE`, `is_heavy` and `static_prefix`; the test
`test_static_prefix_stops_at_a_heavy_command_anywhere_in_a_part` in `tools/test_work.py`.

## Current state

`work.static_prefix('env -i HOME=x cargo test -p c t')` returns the whole command; it should return `''`.

## Plan

Let the `env` alternative in `HEAVY_ANYWHERE` take options before the assignments, for example
`env(?:\s+-[A-Za-z-]+(?:\s+[A-Za-z_]\w*)?)*` (`-i`, `--ignore-environment`, `-u NAME`), and add dec-01be49's form to
the test above.

## Done when

The `[[verify]]` command passes, and `test_work.py` stays green.

## Notes

Tracker lane: `tools/work.py` is one file. Related: gap-de8fdc (verifies that can never pass or never fail).
