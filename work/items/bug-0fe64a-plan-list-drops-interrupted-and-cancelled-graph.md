+++
id = "bug-0fe64a"
kind = "bug"
title = "plan list drops interrupted and cancelled Graph runs: overlay_graph_checkpoint_status ignores those statuses"
status = "open"
triage = "unverified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/plan"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-20ab07"
anchors = ["crates/roko-cli/src/plan.rs::overlay_graph_checkpoint_status"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-20ab07"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib overlay_graph_checkpoint_status"
+++

## Problem

gap-20ab07 made `roko plan status` show interrupted and cancelled runs. `overlay_graph_checkpoint_status` in `plan.rs`, which `plan list` and the plan summaries use, still maps only the other checkpoint statuses, so those runs show their stale `tasks.toml` status.

## Plan

Map interrupted and cancelled checkpoints the way `plan status` now does, and add a test named `overlay_graph_checkpoint_status_*`.

## Done when

- `cargo test -p roko-cli --lib overlay_graph_checkpoint_status` passes, and covers both statuses.

## Notes

- Reported on 2026-10-01 by the worker on gap-20ab07, during the evening close-out round.
