+++
id = "bug-2a2f63"
kind = "bug"
title = "A dropped run's heartbeat task in graph_task_dispatch/watchdog.rs is never aborted"
status = "open"
triage = "unverified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch/watchdog"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-model-truth's report, checked on work/bug-739dcc at 1efb2fddb)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/watchdog.rs"]
lane = "rust-hot"
parent = "spec-edda86"
links = { depends_on = [], blocks = [], related = ["bug-739dcc"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_dropped_run_stops_its_heartbeat' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_dropped_run_stops_its_heartbeat"
+++

## Problem

While an agent runs, the watchdog emits a heartbeat every `AGENT_HEARTBEAT_INTERVAL` from a separate task (`crates/roko-cli/src/graph_task_dispatch/watchdog.rs:473-511` on bug-739dcc's branch). When the run is dropped (a stall cancel, a restart, a shutdown), that task isn't aborted: it keeps ticking, and keeps heartbeating for a run that's gone (wk-model-truth).

## Why it matters

Watchdog and supervision (epic spec-edda86): a heartbeat for a dead run makes the dashboard and the stall detector believe it's alive, and the task leaks. p3.

## Where

The heartbeat task's spawn in `watchdog.rs`.

## Plan

1. Keep the heartbeat's `JoinHandle` in a guard that aborts it on drop, or tie the loop to the run's cancellation token.
2. Add `a_dropped_run_stops_its_heartbeat`.

## Done when

- [ ] Dropping a run stops its heartbeat.
- [ ] The `[[verify]]` command passes.
