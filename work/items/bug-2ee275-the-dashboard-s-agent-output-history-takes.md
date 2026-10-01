+++
id = "bug-2ee275"
kind = "bug"
title = "The dashboard's agent output history takes ring lines only while it is empty"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/tui"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "f4323cf9d"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-4cac0e"
anchors = ["crates/roko-cli/src/tui/state/snapshot.rs", "crates/roko-cli/src/tui/state/mod.rs::AgentOutputHistory", "crates/roko-cli/src/tui/app/event_loop.rs::refresh_snapshot"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-4cac0e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn agent_output_history_takes_later_ring_lines' crates/roko-cli/src/tui/state/tests.rs && grep -qw 'fn full_refresh_keeps_the_plan_set' crates/roko-cli/src/tui/app/tests.rs && cargo test -p roko-cli --lib -- agent_output_history_takes_later_ring_lines full_refresh_keeps_the_plan_set"
+++

## Problem

`snapshot.rs` ingests task-output ring lines into `AgentOutputHistory` only while the history is empty (a `len == 0` guard). Later ring lines never reach the history unless AgentOutput events carry them. After an explicit full refresh, the standalone dashboard also shows the disk's every-plan list until the hub snapshot next changes (`event_loop.rs::refresh_snapshot`).

## Plan

Append new ring lines by sequence, not only into an empty history, and keep the hub's plan set across a full refresh. Add tests named `agent_output_history_takes_later_ring_lines` and `full_refresh_keeps_the_plan_set`.

## Done when

- Both tests pass.

## Notes

- Reported on 2026-10-01 by wk-tuiv, working on bug-4cac0e, during the evening close-out round.
- 2026-10-01 (wk-tuiv): implemented on work/bug-6c11d1; cargo verification deferred to the batch check.
  `AgentOutputHistory::ingest_ring` replaces the `len == 0` guard. It appends only the lines a sliding ring adds
  after its overlap with the ring taken in last, and stops for an agent once records arrive from another path
  (AgentOutput events), noticed when the history's next sequence number moves past what the last ring left.
  That keeps the old guard's purpose, rings stop once events feed the agent; a ring taken in before the first
  event can still overlap it, as it could before. After a full refresh
  (`event_loop.rs::refresh_snapshot` and its async twin), `reapply_hub_snapshot` marks the hub snapshot changed
  and drains it, so the plan lists the disk load replaced come back from the hub at once. The
  `TuiState.agents` comment no longer names the nonexistent agent_pool and agent_output widgets.
  Tests: `agent_output_history_takes_later_ring_lines` (state) and `full_refresh_keeps_the_plan_set` (app).
