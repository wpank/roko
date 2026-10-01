+++
id = "bug-2a2f63"
kind = "bug"
title = "A dropped run's heartbeat task in graph_task_dispatch/watchdog.rs is never aborted"
status = "done"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch/watchdog"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "479bec688"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-model-truth's report, checked on work/bug-739dcc at 1efb2fddb)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/watchdog.rs"]
lane = "rust-hot"
parent = "spec-edda86"
links = { depends_on = [], blocks = [], related = ["bug-739dcc"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_dropped_run_stops_its_heartbeat' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_dropped_run_stops_its_heartbeat"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T12:49:51Z"
by = "coordinator (session 7622b882)"
forced = false
evidence = "Batch 20d gate on fae7133cd, re-checked with the clippy fix on 9f3c184c5 (MAIN 479bec688 has the same crates and portal): check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-agent/cli/compose/core/execution/graph/neuro/runtime/serve; lib tests roko-cli 3282, roko-agent 2282, roko-core 1962, roko-serve 990, roko-compose 561, roko-graph 476, roko-runtime 288, roko-execution 245, roko-neuro 239 all pass; extras: codex/cursor/openai parity 4+4+4 (streaming tests no longer ignored), default_engine 1, C1 1, C7 2, bin 429, graph_task_dispatch loop 10/10, including a_dropped_run_stops_its_heartbeat (the heartbeat lived in the Claude CLI and exec adapters, now AbortOnDropHandle). Merged 335132dd2."
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

## Notes

- Implemented on `work/mt-l3` at `9f26fa0a5`; cargo verification deferred to the batch check. `a_dropped_run_stops_its_heartbeat` (targeted `cargo test` passed; it checks mid-run that the run's tasks are counted), with `a_dropped_claude_run_stops_its_heartbeat` and `a_dropped_exec_run_stops_its_heartbeat`. The leaked task was not in `watchdog.rs`: the watchdog's heartbeat is an interval inside `run_watched` and is dropped with it. The Claude CLI and exec adapters each spawned a heartbeat task per run and aborted it only on paths they handled. Both now hold it in `tokio_util::task::AbortOnDropHandle` (roko-agent takes the workspace tokio-util).
