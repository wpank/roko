+++
id = "bug-583e50"
kind = "bug"
title = "Router success retractions are not journaled, and LinUCB updates cannot be retracted"
status = "open"
triage = "unverified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-learn/cascade"]
created = 2026-10-01
updated = 2026-10-01
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
