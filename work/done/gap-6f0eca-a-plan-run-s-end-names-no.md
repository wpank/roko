+++
id = "gap-6f0eca"
kind = "gap"
title = "A plan run's end names no run: DashboardEvent::RunCompleted has no run id, so /ws never sends run_completed for plan runs"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "M"
subsystem = ["roko-serve/bridge"]
created = 2026-10-12
updated = 2026-10-12
last_verified = 2026-10-12
last_verified_rev = "e282f4973"
source = "hermes-plugin-roko 0.2.0 UI work, roko serve probe 2026-10-11 (scratchpad roko-ui-data.md)"
anchors = ["crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent", "crates/roko-cli/src/graph_execution/plan_runner.rs", "crates/roko-serve/src/lib.rs::dashboard_event_to_server"]
links = { depends_on = [], blocks = [], related = ["bug-bfdb9a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_run_completion_reaches_ws_with_its_run_id' crates/roko-serve/ && cargo test -p roko-serve plan_run_completion_reaches_ws_with_its_run_id"

[closed]
at = 2026-10-12
at_ts = "2026-10-12T11:58:54Z"
commit = "e282f4973"
by = "commit trailer"
executor = "claude-agent"
size = "M"
claimed_at = "2026-10-12T11:02:45Z"
forced = false
evidence = "e282f4973: DashboardEvent::RunCompleted carries the run id (plan runner, TuiBridge, runtime projector; older events still read), and roko serve publishes ServerEvent::RunCompleted {run_id, success, verdict} for each plan run from the path that settled it (the run task for one plan or a set, the cancel handler on abort, a queued run's cancel). Verify passes in the worktree: run_completed_carries_its_run_id (roko-core) and plan_run_completion_reaches_ws_with_its_run_id (roko-serve); runtime projector, status-file and TUI tests pass; clippy -p roko-core -p roko-runtime -p roko-cli -p roko-serve --no-deps -D warnings clean."
+++

## Problem

When a plan run ends, nothing on the live channels names the run. The hub's `DashboardEvent::RunCompleted`
has no run id, so `dashboard_event_to_server` cannot bridge it (bug-bfdb9a left it out for that reason), and
`/ws` sends `run_completed{run_id, success, verdict}` only for prompt runs (`routes/run.rs`). A plan run's end
reaches `/ws` and `/api/events` only as one `plan_completed{plan_id, success}` per plan.

## Why it matters

A client that started a plan run by id (a chat host over `/mcp` `plan_run`, the portal) cannot learn that the
run ended without polling its summary, and a plan set's end has no single event. Completion notices in chat
hosts (hermes-plugin-roko's gateway notice) need this. Goal: live visibility.

## Where

- `crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent` (`RunCompleted`: outcome, duration and cleanup
  only).
- `crates/roko-cli/src/graph_execution/plan_runner.rs`: publishes `RunCompleted` at the end of a run.
- `crates/roko-serve/src/lib.rs::dashboard_event_to_server` lists `RunCompleted` as not bridged.

## Current state

The per-run index adds `run_id` to every row it writes, but the live hub event itself has none.

## Plan

1. Add `run_id: Option<String>` (serde default, skipped when absent) to `DashboardEvent::RunCompleted`, and set it
   where the plan runner publishes it.
2. Bridge `RunCompleted` with a run id to `ServerEvent::RunCompleted { run_id, success, verdict }`, success when
   the outcome is `succeeded`; keep events without one unbridged.
3. Update the other constructors, and test the bridge.

## Done when

- A plan run's end reaches `/ws` as `run_completed` with its run id, and the hub event carries the run id.
- The `[[verify]]` test passes.
