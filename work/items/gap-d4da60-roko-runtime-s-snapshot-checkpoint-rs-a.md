+++
id = "gap-d4da60"
kind = "gap"
title = "roko-runtime's snapshot_checkpoint.rs (a StateHub checkpoint writer for statehub-snapshot.json) is outside the module tree and never compiled, while state_snapshot.rs already persists the dashboard projection"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-runtime/state-hub"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "gap-7ebc4f"
discovered_from = "gap-7ebc4f (worker, 2026-10-17): 9c6ec420c lists it among the modules not yet declared in any lib.rs"
anchors = ["crates/roko-runtime/src/snapshot_checkpoint.rs::start_checkpoint_writer", "crates/roko-runtime/src/state_snapshot.rs::load_durable_dashboard_projection", "crates/roko-runtime/src/lib.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-7ebc4f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "( test ! -e crates/roko-runtime/src/snapshot_checkpoint.rs || grep -q \"mod snapshot_checkpoint;\" crates/roko-runtime/src/lib.rs ) && cargo test -p roko-runtime --lib snapshot_checkpoint"
+++

## Problem

`crates/roko-runtime/src/snapshot_checkpoint.rs` (309 lines: `load_checkpoint`, `flush_checkpoint`,
`start_checkpoint_writer`, `bootstrap_from_checkpoint`, `CheckpointWriterHandle`, `checkpoint_path_for_workdir`) came
in with commit `9c6ec420c` (2026-09-25), whose message lists it among the "new modules not yet declared in any
lib.rs". It's still undeclared: `grep -n 'mod snapshot_checkpoint' crates/roko-runtime/src/lib.rs` prints nothing.
So the file isn't compiled or tested, and nothing calls it. As written, it would write StateHub's snapshot to
`.roko/state/statehub-snapshot.json` every `CHECKPOINT_INTERVAL` and restore StateHub from that file at startup.

The compiled `state_snapshot.rs` already persists a durable dashboard projection to `.roko/state/state-snapshot.json`
(`STATE_SNAPSHOT_RELATIVE_PATH`, `load_durable_dashboard_projection`).

## Why it matters

Two designs for one job, one of them never built, mislead anyone who reads `src/`. The orphan may no longer
compile against today's `StateHub`. Goal `tooling`.

## Where

- `crates/roko-runtime/src/snapshot_checkpoint.rs`: the orphan. It imports
  `roko_core::dashboard_snapshot::DashboardSnapshot` and `crate::state_hub::SharedStateHub`.
- `crates/roko-runtime/src/state_snapshot.rs::load_durable_dashboard_projection`: the live persistence path.
- `crates/roko-runtime/src/lib.rs`: the module list.

## Current state

Not compiled since `9c6ec420c`. Found while parking the channel layer (gap-7ebc4f). Unlike the other orphans from
that commit, it isn't channel code, so the `channels` feature isn't the place for it.

## Plan

1. Compare it with `state_snapshot.rs`. If the durable projection already covers cold-start recovery, delete
   `snapshot_checkpoint.rs` and say so in the commit.
2. Otherwise declare it, make it compile and pass its own tests, and file wiring it as a separate item.

## Done when

The `[[verify]]` passes: the file is gone, or it is declared in `lib.rs` and roko-runtime still compiles.

## Notes

Keep `.roko/state/state-snapshot.json` and its readers unchanged.
