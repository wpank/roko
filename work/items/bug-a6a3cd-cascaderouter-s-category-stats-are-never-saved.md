+++
id = "bug-a6a3cd"
kind = "bug"
title = "CascadeRouter's category_stats are never saved, so per-category pass rates restart empty in every process"
status = "open"
triage = "unverified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-learn/cascade"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-583e50"
anchors = ["crates/roko-learn/src/cascade_router.rs"]
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
