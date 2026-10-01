+++
id = "bug-583e50"
kind = "bug"
title = "Router success retractions are not journaled, and LinUCB updates cannot be retracted"
status = "open"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-learn/cascade"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "825d45f97"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-b95d94"
anchors = ["crates/roko-learn/src/cascade_router.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-b95d94"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-learn --lib retraction_survives_a_crash"
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
