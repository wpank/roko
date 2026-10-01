+++
id = "bug-60ccba"
kind = "bug"
title = "A snapshot's RunCompleted leaves the run's running tasks active with no end time"
status = "open"
triage = "unverified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-core/dashboard_snapshot"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-8a1fb3"
anchors = ["crates/roko-core/src/dashboard_snapshot.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-8a1fb3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-core --lib run_completed_ends_running_tasks"
+++

## Problem

A snapshot's RunCompleted ends the running plans' clocks, but the run's running tasks stay active, with no end time.

## Plan

End them too, as interrupted or unknown, never passed. Add a test named `run_completed_ends_running_tasks`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-01 by wk-streams, working on gap-8a1fb3, during the evening close-out round.
