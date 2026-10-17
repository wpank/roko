+++
id = "bug-808700"
kind = "bug"
title = "roko learn cannot run while roko serve runs: serve holds the workspace lock exclusively and learn's read-only subcommands exit 2, as doctor did before bug-1f5406"
status = "open"
triage = "verified"
severity = "p3"
goal = "hermes"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "round-v6-dogfood-will#R3-8"
discovered_from = "agent:dogfood-will (round v6 run 3, 2026-10-16)"
anchors = ["crates/roko-cli/src/commands/learn.rs::dispatch_learn", "crates/roko-cli/src/doctor.rs::serve_holding_workspace"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-1f5406"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn learn_reads_beside_the_serve_that_holds_the_workspace' crates/roko-cli/ && cargo test -p roko-cli learn_reads_beside_the_serve_that_holds_the_workspace"
+++

## Problem

While roko serve runs in a workspace, every `roko learn` subcommand fails. Serve runs all day when Hermes uses roko.

```
$ roko learn loops
error: Another roko process holds an exclusive lock on this workspace.
  hint: wait for it to finish or kill it.
(exit 2)
```

This happened in round v6's dogfood run 3 (2026-10-16, roko 47b3cdd20), for `roko learn loops` and `roko learn route`.
After serve stopped, both printed normally. So the one CLI view of the learning loops cannot be used while the loops
are running, which is when it is wanted.

## Why it matters

- Goal `hermes`: the Hermes workflow keeps serve up.
- Showing the learning depends on `roko learn …` output, in guides and in evidence.
- bug-1f5406 fixed the same problem for `roko doctor` (64c6b4394).

## Where

- `crates/roko-cli/src/commands/learn.rs::dispatch_learn` takes `acquire_workspace_lock_shared` for every subcommand
  before dispatching it.
- `crates/roko-cli/src/doctor.rs::serve_holding_workspace`, used by the doctor path in
  `crates/roko-cli/src/commands/util.rs`, is bug-1f5406's fix. When the shared lock is refused and its owner is this
  workspace's serve, doctor reads beside serve without the lock and says so.
- `crates/roko-cli/src/workspace_lock.rs::acquire_workspace_lock_shared`.

## Current state

Every learn subcommand takes the shared lock, the ones that write included. The writers must keep refusing beside
serve, for example `learn experiments create` and `conclude`. Check each subcommand.

## Plan

1. Split learn's subcommands into readers and writers.
   - Readers include `loops`, `router`/`route`, `all`, `efficiency`, `episodes`, `gates`, `reflexes`,
     `knowledge-stats`, `inspect`, `self-model replay` and `econ prices`. Check the rest one by one.
   - A subcommand that saves state is a writer.
2. For the readers, do what doctor does: when the shared lock is refused and `serve_holding_workspace` names this
   workspace's serve, read without the lock and print one note line saying that roko serve is running.
3. Add the test `learn_reads_beside_the_serve_that_holds_the_workspace`.

## Done when

- The `[[verify]]` passes.
- `roko learn loops` prints the loop census while serve runs.

## Notes

- A reader beside serve can catch a JSONL file mid-append. The learn readers should skip a torn last line rather than
  fail, so check that they do.
- Evidence: finding R3-8 in `tmp/hermes-roko-v6/findings/dogfood-will.md`.
