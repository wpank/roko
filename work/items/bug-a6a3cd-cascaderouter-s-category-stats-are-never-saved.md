+++
id = "bug-a6a3cd"
kind = "bug"
title = "CascadeRouter's category_stats are never saved, so per-category pass rates restart empty in every process"
status = "open"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-learn/cascade"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "c7560e213"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-583e50"
anchors = ["crates/roko-learn/src/cascade_router.rs", "crates/roko-learn/src/cascade/persistence.rs::merge_learning"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-583e50"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-learn --lib category_stats_survive_a_save"
+++

## Problem

`CascadeRouter::category_stats` is never written to `cascade-router.json`, so Stage 2's per-category pass-rate delta starts empty in every process.

## Plan

Persist and merge them like the other learned state. Add a test named `category_stats_survive_a_save`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-01 by wk-learn2, working on bug-583e50, during the evening close-out round.
- 2026-10-02 (wk-settle): implemented on work/bug-f9ae3e; cargo verification deferred to the batch check.
  `CascadeSnapshot` gains `category_stats` (by model, then category; empty in older snapshots). The router saves
  its counts, `merge_learning` adds what it learned since its baseline, as for the confidence counters, and a load
  restores them under the slugs they were recorded for. A success a relabel retracted since the base leaves the
  saved counts. Tests: `category_stats_survive_a_save`, `category_stats_merge_keeps_retractions`.
- With bug-583e50 (wk-learn2, gate 6d): its WAL replay of a retraction (`replay_retraction`) restores only the
  confidence half, since category counts weren't persisted then; a crash before the save now loses the category
  half of a journaled retraction.
