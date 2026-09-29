+++
id = "bug-84de98"
kind = "bug"
title = "LearningRuntime::open replays a running writer's unsaved model-call observations, which that writer later saves again"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-learn/runtime_feedback", "roko-learn/wal"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-router2's report on bug-605a8a, branch work/bug-605a8a)"
anchors = ["crates/roko-learn/src/runtime_feedback/mod.rs::replay_and_open_wal", "crates/roko-learn/src/wal.rs::folded_model_call_ids", "crates/roko-learn/src/model_call_feedback.rs"]
lane = "rust-hot"
parent = "spec-6ac537"
links = { depends_on = ["bug-605a8a"], blocks = [], related = ["bug-605a8a", "bug-7a2630", "bug-8b0d0a", "find-0dc1d5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_running_writers_unsaved_observations_are_counted_once' crates/roko-learn/src/ && cargo test -p roko-learn --lib a_running_writers_unsaved_observations_are_counted_once"
+++

## Problem

`replay_and_open_wal` (`crates/roko-learn/src/runtime_feedback/mod.rs:181`) replays every `ModelCallObservation` in `.roko/learn/wal.jsonl` whose id has no fold marker (`wal::folded_model_call_ids`, :207-245). It then saves the router and truncates the WAL (:249 onward).

The WAL is shared. A writer that is still running, such as serve between two saves, journals its observations there before it saves them. When another process opens a `LearningRuntime` in that window, it replays those observations into its router and saves them. The running writer later saves the same observations again, so they are counted twice.

On `work/bug-605a8a` (7769a5ae4), saves merge each process's learning into the snapshot by adding its A and b deltas, so the second save really adds them again.

## Why it matters

Cybernetic core (epic spec-6ac537): routing learns from double-counted outcomes, which skews each arm's statistics towards whichever models the busy writer used.

## Where

- `replay_and_open_wal`, and the fold markers in `crates/roko-learn/src/wal.rs`.
- The model-call feedback surface that journals the observations (`model_call_feedback.rs`).

## Current state

At ad391f99a replay skips only folded ids. Nothing tells replay whether an unfolded entry belongs to a live writer.

## Plan

1. Only replay what no live process will save. Options:
   - record the writer's pid and start time with each entry, and skip entries of a live writer;
   - take an advisory lock on the WAL while a writer runs, and replay only unlocked segments;
   - have each writer fold (mark) its own entries when it saves, and let replay re-check folds after the writer exits.
2. Add `a_running_writers_unsaved_observations_are_counted_once`: writer A journals without saving, B opens (and replays), A saves, and the snapshot counts each observation once.

## Done when

- [ ] An observation reaches the router's state exactly once, whatever the interleaving of opens and saves.
- [ ] The `[[verify]]` command passes.

## Notes

- Land with or after bug-605a8a, whose merge-on-save semantics this interacts with.
- Implemented on `work/bug-84de98` at `c48c6a2ad`; cargo verification deferred to the batch check. Each writer journals into its own locked segment under `.roko/learn/wal/`, and an opener replays only segments whose lock it can take. A crash between a save and the truncation after it still replays that segment once more, the same window the fold markers had.
