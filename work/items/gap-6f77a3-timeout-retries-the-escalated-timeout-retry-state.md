+++
id = "gap-6f77a3"
kind = "gap"
title = "timeout_retries (the escalated-timeout retry state) is memory-only and resets on resume"
status = "open"
triage = "unverified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-34b2ed"
anchors = ["crates/roko-cli/src/graph_task_dispatch/retry_feedback.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-34b2ed"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib timeout_retries_survive_resume"
+++

## Problem

gap-34b2ed persisted per-task spend and turn caps in `retry-feedback.json`. The escalated-timeout retry count (`timeout_retries`, which grows a retry's timeout 1.5x) is still in memory only, so a resumed run restarts the escalation from the base timeout.

## Plan

Persist it beside spend and turn_caps, serde-defaulted. Add a test named `timeout_retries_survive_resume`.

## Done when

- `cargo test -p roko-cli --lib timeout_retries_survive_resume` passes.

## Notes

- Reported on 2026-10-01 by the worker on gap-34b2ed, during the evening close-out round.
