+++
id = "bug-808700"
kind = "bug"
title = "roko learn cannot run while roko serve runs: serve holds the workspace lock exclusively and learn's read-only subcommands exit 2, as doctor did before bug-1f5406"
status = "done"
triage = "verified"
severity = "p3"
goal = "hermes"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-10-17
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "99db58d80"
source = "round-v6-dogfood-will#R3-8"
discovered_from = "agent:dogfood-will (round v6 run 3, 2026-10-16)"
anchors = ["crates/roko-cli/src/commands/learn.rs::dispatch_learn", "crates/roko-cli/src/doctor.rs::serve_holding_workspace"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-1f5406"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn learn_reads_beside_the_serve_that_holds_the_workspace' crates/roko-cli/ && cargo test -p roko-cli learn_reads_beside_the_serve_that_holds_the_workspace"

[closed]
at = 2026-11-12
at_ts = "2026-11-12T10:49:49Z"
commit = "99db58d80"
by = "commit trailer"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-11-12T10:08:22Z"
forced = false
evidence = "99db58d80. crates/roko-cli: workspace_lock::acquire_read_lock takes the shared lock for a command that only reads, or reads beside the workspace's own roko serve (doctor::serve_holding_workspace), as bug-1f5406 did inline for doctor, which now uses it too; dispatch_learn uses it for every view and prints one stderr note beside serve, while the writers (rollback, probes clear, loops canary/fault/clear, experiments create/conclude; learn_only_reads) keep the shared lock and stay refused. Red first on 0f7ef44ea (the read lock refusing beside serve): learn_reads_beside_the_serve_that_holds_the_workspace failed; green: it, learn_writers_keep_the_workspace_lock and doctor_beside_serve_says_so_and_leaves_lock_files_alone pass; roko-cli lib 4100/0, bins 10/0 and 480/0, roko-serve lib 1091/0; clippy -D warnings and nightly fmt clean (tmp/hermes-roko-v8/roko-learnguard/logs/808700-chain.log). Live, $0 (bin/808700-live.sh, a real roko serve of 99db58d80 on port 6690 over a scratch copy of Undertow's workspace, scratch HOME): roko learn loops and roko learn all exit 0 beside it with the note, roko learn probes clear --all is refused (exit 2), and after serve stop roko learn loops runs without the note."
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

Checked 2026-10-27: 2026-10-26 sweep: still open; doctor's beside-serve pattern (util.rs:1495-1498) has not been ported to learn.rs::dispatch_learn.
