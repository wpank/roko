+++
id = "bug-a6a3cd"
kind = "bug"
title = "CascadeRouter's category_stats are never saved, so per-category pass rates restart empty in every process"
status = "done"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-learn/cascade"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "4dc345a29"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-583e50"
anchors = ["crates/roko-learn/src/cascade_router.rs", "crates/roko-learn/src/cascade/persistence.rs::merge_learning"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-583e50"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-learn --lib category_stats_survive_a_save"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T00:39:01Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T22:34:31Z"
forced = false
evidence = "Gate 6e on ddf47dbd6 plus its fixes, re-run at 3ac297a00 and merged as 4dc345a29 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 10 crates; lib tests pass (roko-cli 3420, roko-agent 2289, roko-core 1984, roko-learn 1230, roko-serve 1013, roko-graph 488, roko-conductor 316, roko-acp 220, roko-execution 193, roko-dreams 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp integration, smoke, graph_plan_callers, graph_timeout_matrix and plan_conversion pass; bin 445; scripts/test_run_evidence_graph.py 9/9 against the gate binary; Cargo.lock unchanged. Implemented in this round; the item's notes name the change and its test."
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
- 2026-10-02 (wk-settle): the WAL now carries the category too (after merging gate 6d, `a788dfd8d`). The
  `CascadeObservation`, `ModelCallObservation` and `SuccessRetraction` entries name the task category whose counts
  they moved (optional; older entries have none). `LearningRuntime`'s observations and the journal's task and
  override outcomes and retractions fill it in, and the replay moves the category counts with the observation, or
  applies both halves of a retraction (`CascadeRouter::retract_success`). Test:
  `journaled_category_counts_survive_a_crash`.
