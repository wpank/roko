+++
id = "bug-2ee275"
kind = "bug"
title = "The dashboard's agent output history takes ring lines only while it is empty"
status = "open"
triage = "unverified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/tui"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-4cac0e"
anchors = ["crates/roko-cli/src/tui/state/snapshot.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-4cac0e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib agent_output_history_takes_later_ring_lines"
+++

## Problem

`snapshot.rs` ingests task-output ring lines into `AgentOutputHistory` only while the history is empty (a `len == 0` guard). Later ring lines never reach the history unless AgentOutput events carry them. After an explicit full refresh, the standalone dashboard also shows the disk's every-plan list until the hub snapshot next changes (`event_loop.rs::refresh_snapshot`).

## Plan

Append new ring lines by sequence, not only into an empty history, and keep the hub's plan set across a full refresh. Add tests named `agent_output_history_takes_later_ring_lines` and `full_refresh_keeps_the_plan_set`.

## Done when

- Both tests pass.

## Notes

- Reported on 2026-10-01 by wk-tuiv, working on bug-4cac0e, during the evening close-out round.
