+++
id = "gap-775aa6"
kind = "gap"
title = "roko learn rollback router leaves the WAL segment, which can replay over the restored file"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-13 follow-up reports 2026-10-04 (PK72 gap-309b26, task 8139)"
discovered_from = "gap-309b26 (open, unmerged on work/gap-309b26 at b4c8ae979)"
anchors = ["crates/roko-learn/src/model_call_feedback.rs::load_recovered_router", "crates/roko-learn/src/model_call_feedback.rs::ModelCallJournal"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn router_rollback_survives_a_stale_wal_segment' crates/roko-cli/ crates/roko-learn/ && cargo test -p roko-cli router_rollback_survives_a_stale_wal_segment"
+++

## Problem

`roko learn rollback router --to <v>` writes the kept version back into `cascade-router.json`
but leaves the router's WAL segment in place, so a segment left by a crashed run can replay on
top of the restored file and silently undo the rollback. This task (8139) is implemented on the
unmerged branch `work/gap-309b26` (checked at `b4c8ae979`, not yet on `main`), in
`crates/roko-cli/src/commands/learn_commits.rs`. Its rollback path is
`RouterFile::restore(&mut self, snapshot: &[u8])` (`learn_commits.rs:208-221`): it parses the
kept version's JSON and rewrites `cascade-router.json` under `roko_fs::with_locked_json_transaction`
— nothing else. It never calls `recover_wal` or otherwise touches or truncates the router's WAL
segment file.

That WAL exists for a real reason, and is real production machinery on `main` already:
`ModelCallJournal` (`crates/roko-learn/src/model_call_feedback.rs:338-358`) "appends each
observation to its own WAL segment beside the snapshot before applying it, and truncates the
segment once a saved snapshot holds its observations" (its own doc comment, lines 344-347) — a
crash-recovery mechanism (find-0dc1d5) for observations not yet durably saved into the
snapshot. `load_recovered_router` (`model_call_feedback.rs:649-656`), the function every live
router-loading path on `main` uses (`chat_session.rs`, `dispatch_v2.rs`, `serve_runtime.rs`,
`runner/types.rs`, `runtime_feedback/routing.rs`, `roko-serve`'s `dispatch.rs`/`state.rs`/
`service_factory.rs`/`routes/gateway.rs`, `roko-acp`'s `bridge_events/experiments.rs`),
unconditionally calls `runtime_feedback::recover_wal(&paths)` *before* loading whatever snapshot
is on disk. So: a run crashes after journaling observations to the WAL segment but before its
own snapshot save; its segment sits on disk with unsaved observations. Someone rolls the router
back to an earlier kept version with `roko learn rollback router --to <v>`, which only rewrites
`cascade-router.json` and never looks at the WAL segment. The next time anything loads the
router — the very next `roko run`, `roko serve`, etc. — `load_recovered_router` replays that
stale segment on top of the just-restored file, reapplying observations from after the version
the rollback was meant to discard.

## Why it matters

Goal: cybernetic, M1 controller / guarded commit rollback (backlog 8139). A rollback exists
specifically so a bad router state can be discarded with confidence. If a leftover WAL segment
can silently reintroduce exactly what was discarded, the rollback command's guarantee is false
in a case that's realistic, not contrived: a crashed run is the normal scenario this WAL exists
to survive, and it is also the normal scenario that creates the stale segment.

## Where

- `work/gap-309b26` (unmerged; checked at `b4c8ae979`): `crates/roko-cli/src/commands/learn_commits.rs::RouterFile::restore`,
  `::rollback` (8139's implementation — this branch does not exist on `main` yet, so it cannot
  be anchored from the main checkout).
- `crates/roko-learn/src/model_call_feedback.rs::ModelCallJournal`, `::load_recovered_router`,
  `::save` (on `main`; the WAL this rollback needs to also clear).
- `crates/roko-learn/src/runtime_feedback/mod.rs::recover_wal` (on `main`; the replay path).

## Current state

The rollback is implemented and tested for the snapshot-rewrite case (`learn_commits.rs`'s own
test, `learn_rollback_restores_router_version`), but that test never journals a WAL segment
first, so it doesn't exercise this interaction.

## Plan

1. When `rollback` targets the router store, also clear (truncate or delete) its WAL segment —
   the same segment `ModelCallJournal`/`recover_wal` would otherwise replay — so nothing is left
   to reapply on top of the restored snapshot.
2. Add a regression test: journal an observation to the WAL segment (without saving a snapshot
   that absorbs it, simulating a crash), roll back to an earlier version, then call
   `load_recovered_router` and assert the rolled-back state survives rather than being
   overwritten by the replayed segment.

## Done when

- A WAL segment present at rollback time cannot reintroduce state the rollback discarded.
- The `[[verify]]` command passes.

## Notes

- 2026-10-04 (wave-13 follow-up, PK72 gap-309b26, task 8139): premise confirmed by reading
  `RouterFile::restore` on `work/gap-309b26` at `b4c8ae979` (unmerged) and cross-referencing
  `ModelCallJournal`/`load_recovered_router`/`recover_wal` on `main` at `b7ad508ce`, whose
  production call sites (`chat_session.rs`, `dispatch_v2.rs`, `serve_runtime.rs`, `roko-serve`'s
  dispatch/state/service_factory/gateway route, `roko-acp`'s bridge_events) confirm the replay
  path is live, not dead code. Filed as a standalone item rather than a note on `gap-309b26`,
  since that package item's scope is implementing 8139, not this follow-on interaction; link
  the two when `gap-309b26` is picked back up or merged.
