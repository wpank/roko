+++
id = "bug-583e50"
kind = "bug"
title = "Router success retractions are not journaled, and LinUCB updates cannot be retracted"
status = "done"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-learn/cascade"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "39feebc07"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-b95d94"
anchors = ["crates/roko-learn/src/cascade_router.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-b95d94"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-learn --lib retraction_survives_a_crash"

[closed]
at = 2026-10-02
at_ts = "2026-10-01T23:45:17Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T18:51:09Z"
forced = false
evidence = "Gate 6d on 9eacfde5f plus its fixes, re-checked at c9e78d12d and merged as 39feebc07 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean; lib tests pass (roko-cli 3407, roko-agent 2276, roko-core 1980, roko-learn 1228, roko-serve 1012, roko-acp 219, roko-compose 562, roko-execution 192, roko-gateway 43); all eight canaries with the new default-isolation canary, golden_path_suite, secret_canary, C2, graph_plan_callers, cost_dedup, phase0_wiring, run_serve_share, graph_timeout_matrix (6), plan_prepare_full and the gateway pipeline contract pass; bin tests pass; scripts/test_run_evidence_graph.py 9/9 against the gate binary; Cargo.lock unchanged; route snapshot matches. Implemented in this round; the item's notes name the change and its test."
+++

## Problem

gap-b95d94 added `CascadeRouter::retract_success`. The retraction isn't journaled, so a crash before the run saves the router loses it while the journal replays the original success. LinUCB updates have no retraction at all.

## Plan

Journal retractions as the successes they undo are journaled, and add a LinUCB retraction (or document why the contextual bandit is exempt). Add a test named `retraction_survives_a_crash`.

## Done when

- `cargo test -p roko-learn --lib retraction_survives_a_crash` passes.

## Notes

- Reported on 2026-10-01 by wk-learn2, working on gap-b95d94, during the evening close-out round.
- 2026-10-01 (wk-learn2): implemented on work/gap-14f08e; cargo verification deferred to the batch check.
  `ModelCallJournal::retract_success` journals a new `WalEntry::SuccessRetraction` before applying it, and WAL
  recovery replays it in order (`CascadeRouter::replay_retraction`). The hindsight sink retracts through the run's
  journal. Two more gaps turned up and are covered: a router save merged only gains, so a retraction of a success an
  earlier run saved was dropped at every save (`merge_learning` now carries it), and the retraction skipped the
  confidence stats of an operator override, which counts there like any routing outcome. LinUCB is exempt, as
  `CascadeRouter::retract_success` documents: the relabel has neither the dispatch-time features nor the
  EWC-regularized reward the update applied. Category stats aren't in the snapshot, so they stay in memory.
  Tests: `retraction_survives_a_crash`, `a_saved_success_retracted_by_a_later_run_stays_retracted`.
