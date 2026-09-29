+++
id = "bug-7a2630"
kind = "bug"
title = "WAL replay drops entries for models the router doesn't track and then truncates wal.jsonl, and serve never truncates it"
status = "open"
triage = "unverified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-learn/runtime_feedback", "roko-learn/wal"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-router2's report on bug-605a8a, branch work/bug-605a8a)"
anchors = ["crates/roko-learn/src/runtime_feedback/mod.rs::replay_and_open_wal", "crates/roko-learn/src/cascade_router.rs::replay_observation", "crates/roko-learn/src/wal.rs"]
lane = "rust-hot"
parent = "spec-6ac537"
links = { depends_on = [], blocks = [], related = ["bug-605a8a", "bug-84de98", "bug-8b0d0a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn wal_replay_keeps_entries_for_untracked_models' crates/roko-learn/src/ && grep -rqw 'fn the_wal_is_truncated_after_every_save' crates/roko-learn/src/ && cargo test -p roko-learn --lib wal_replay_keeps_entries_for_untracked_models && cargo test -p roko-learn --lib the_wal_is_truncated_after_every_save"
+++

## Problem

Two WAL problems:

- **Lost entries.** `replay_and_open_wal` (`runtime_feedback/mod.rs:181`) replays each entry through `CascadeRouter::replay_observation`. That method looks up the entry's model slug and skips entries whose slug the current router doesn't track (`cascade_router.rs:1356-1360`). After replay, the runtime saves and truncates `wal.jsonl` (mod.rs:249 onward). A process that opens the runtime with a narrower model list, such as a plan whose config names fewer models, therefore drops every other model's unsaved observations for good.
- **Unbounded growth.** Only `LearningRuntime`'s open path truncates the WAL. serve journals model-call observations into it and saves the router, but never truncates it, so `wal.jsonl` grows for as long as serve runs, and every later open replays the whole history.

## Why it matters

Cybernetic core (epic spec-6ac537): learning is lost silently when router configs differ, which is the situation bug-605a8a is about, and replay time and disk grow without bound on a long-running serve.

## Where

- `replay_and_open_wal` and `replay_observation`.
- serve's journaled router (on `work/bug-605a8a`, d32a4609c: one journaled cascade router for the gateway, feedback and dispatch).

## Current state

At ad391f99a both behaviours hold. bug-605a8a's branch restores LinUCB arms by slug from snapshots (0651270df), but WAL replay still drops untracked slugs.

## Plan

1. Keep entries the current router can't apply: write them back to the WAL, or into the snapshot's per-slug state (as bug-605a8a does for arms), before truncating.
2. Truncate, or rotate, the WAL after each successful save in every writer, including serve, keeping entries newer than the save.
3. Add two tests: `wal_replay_keeps_entries_for_untracked_models` (a narrower router opens, and a wider one later still sees the entries) and `the_wal_is_truncated_after_every_save`.

## Done when

- [ ] No WAL entry is lost because of the opener's model list, and the WAL stays bounded while serve runs.
- [ ] The `[[verify]]` command passes.
