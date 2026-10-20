+++
id = "bug-0b997a"
kind = "bug"
title = "A live Graph run's index holds agent rows until the next lifecycle row, so live readers see nothing during an agent turn"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/graph-execution"]
created = 2026-10-12
updated = 2026-10-12
last_verified = 2026-10-12
last_verified_rev = "4a4bf7ab6"
source = "hermes-plugin-roko e2e 2026-10-12, live per-run stream of run 22cfb25f (roko serve 2448626a4)"
anchors = ["crates/roko-cli/src/graph_execution/event_log.rs::WorkspaceLogWriter", "crates/roko-cli/src/runner/persist.rs::append_buffered_run_index"]
links = { depends_on = [], blocks = [], related = ["bug-1f41f0"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn agent_rows_reach_the_index_while_the_run_is_live' crates/roko-cli/ && cargo test -p roko-cli agent_rows_reach_the_index_while_the_run_is_live"

[closed]
at = 2026-10-12
at_ts = "2026-10-12T13:22:32Z"
commit = "4a4bf7ab6"
by = "claude (session 9cf6b7db, hermes-plugin-roko)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-12T12:59:18Z"
forced = false
evidence = "4a4bf7ab6: WorkspaceLogWriter flushes the run index for lifecycle rows and for any row once its last flush is at least INDEX_FLUSH_INTERVAL (1 s) old; durability unchanged. Verify passes in the worktree: agent_rows_reach_the_index_while_the_run_is_live; the event-log tests and per_run_stream_carries_a_graph_runs_events pass; clippy -p roko-cli -p roko-serve --no-deps -D warnings clean."
+++

## Problem

A running Graph run's per-run index (`.roko/events-by-run/<sha256(run id)>.jsonl`) holds every row that is not a
plan, task, gate or run lifecycle event in a 64 KiB buffer until the next lifecycle row flushes it. An agent turn
emits no lifecycle row, so its `agent_spawned`, `agent_heartbeat` and `agent_output` rows stay invisible for the
whole turn. Observed live on 2026-10-12 (roko serve `2448626a4`, run 22cfb25f): following
`GET /api/runs/{id}/events/stream`, `task_started` arrived at +1.1 s, then nothing until +41.7 s, when
`agent_spawned`, 16 heartbeats, the agent's output and its completion arrived together with the gate result.

## Why it matters

Everything that follows a live run reads that index: the per-run stream (bug-1f41f0), `GET /api/runs/{id}/events`,
the run detail, and chat hosts' run cards (hermes-plugin-roko folds the index into its cards). During an agent turn,
usually the longest part of a run, they all show the run as stalled. Heartbeats exist to show liveness, and the
buffer hides them.

## Where

- `crates/roko-cli/src/graph_execution/event_log.rs::WorkspaceLogWriter::record`: flushes the index only when
  `is_lifecycle_event`.
- `crates/roko-cli/src/runner/persist.rs::append_buffered_run_index`: the per-run `BufWriter` (64 KiB).

## Current state

Unfixed at `2448626a4`. Lifecycle rows are synced and flushed as they are written; the rest wait for the next one.

## Plan

1. In `WorkspaceLogWriter::record`, also flush the index when the last flush was at least a second ago, so agent
   rows reach readers within about a second of the previous flush. Durability is unchanged: only lifecycle rows are
   synced.
2. Test: an agent row published more than the interval after the last flush is readable from the index while the
   run is still recording.

## Done when

`agent_rows_reach_the_index_while_the_run_is_live` passes (the `[[verify]]` command).

## Notes

Found by following a live run's per-run stream in the hermes-plugin-roko end-to-end test of 2026-10-12.
