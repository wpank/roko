+++
id = "reg-cbfff6"
kind = "regression"
title = "Graph engine stores its event sink but never emits to it"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["roko-graph/engine", "roko-cli/state-hub"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "8a3c530af"
source = "gaps-md#batch-2026-09-05-engine-convergence/statehubgrapheventsink"
discovered_from = "doc:tmp/work-management/01-gaps-md-audit.md"
anchors = ["crates/roko-graph/src/engine.rs::with_event_sink", "crates/roko-graph/src/engine.rs:327", "crates/roko-cli/src/graph_execution/plan_runner.rs:2016", "crates/roko-cli/src/runner/types.rs:2662", "crates/roko-cli/src/runner/types.rs:2717"]
links = { depends_on = [], blocks = [], related = ["gap-7f5eb6", "gap-d40bc0", "gap-8a1fb3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = '''grep -n 'event_sink' crates/roko-graph/src/engine.rs | grep -vE 'event_sink: |fn with_event_sink|self\.event_sink = Some' | grep -q .'''

[closed]
at = 2026-10-01
at_ts = "2026-10-01T15:16:13Z"
by = "coordinator (session 7622b882)"
claimed_at = "2026-10-01T09:06:26Z"
forced = false
evidence = "Batch 20f gate on 2ff1b7891 (MAIN has the same crates and portal): check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-agent/cli/core/graph/serve; lib tests roko-cli 3309, roko-agent 2296, roko-core 1963, roko-serve 992, roko-graph 480 pass; extras: all eight canaries + golden_path_suite + secret_canary 11/11 + C2 2/2 + worktree_task_diff + default_engine pass, bin 429, graph_task_dispatch suite at --test-threads=32 passed 10 of 10, including the_event_sink_receives_node_lifecycle_events. Merged (work/reg-cbfff6 fc2430b99)."
+++

The 2026-09-05 engine-convergence batch recorded `StateHubGraphEventSink` as "wired into all graph execution paths". `GraphEngine` stores the sink (`crates/roko-graph/src/engine.rs:323`, set by `with_event_sink` at `:444`), but no engine code reads it, so the engine itself emits no `GraphExecutionEvent`s. Dashboards only get what the CLI host layer bridges separately, and they get it in batches (see the related items). `RunConfig.http_event_sink` is also always `None` (`crates/roko-cli/src/runner/types.rs:2662`, `:2717`; `crates/roko-cli/src/commands/do_cmd.rs:949`). The CLI-to-serve hub bridge is tracked in gap-7f5eb6.

Fix: have the engine emit node lifecycle events (start, complete, fail, skip, with timestamps) through the stored sink. Add a test in which a sink attached with `with_event_sink` receives them during a real run.

Verified 2026-09-28 (static check against 3d0ee4d02): GraphEngine still only stores the sink: `event_sink` appears in roko-graph solely as the field (engine.rs:327), its None initializer (:353) and the with_event_sink setter (:448-449); nothing in crates/roko-graph/src reads or emits through it (the item's [[verify]] grep still finds no other use). RunConfig.http_event_sink is still only ever None (runner/types.rs:2662, :2717); the do_cmd.rs:949 anchor is stale because do_cmd.rs no longer references RunConfig/http_event_sink.

## Notes

Implemented on `work/reg-cbfff6` at `64c96abab`; cargo verification deferred to the batch check.

The engine publishes `NodeStarted` when a node's cell starts, and `NodeCompleted`, `NodeFailed` or `NodeSkipped` once the node settles, on every path that has the sink: `execute` (`execute_at_tick_validated`), the ready queue (`execute_ready_queue`, used by `execute` and `start` when `max_concurrent_nodes > 1`) and `start`'s sequential run (`execute_with_status_tracking_sequential`). `publish_settled` publishes the results pushed since the last call, so no push site needed its own call. A completed `task-executor` node carries the bug-71a5e6 `outcome` its gate verdict earns, `unverified` without a verdict; other cells carry none. A node skipped because a node it depends on failed names that node in `reason`. Tests: `the_event_sink_receives_node_lifecycle_events` (`execute` and `start`, one and four concurrent nodes) and `a_completed_task_node_carries_its_gate_outcome`.

What it does not do:
- Timestamps: the event schema has `seq` and `elapsed_ms` but no wall-clock field. The production sink, `GraphEventLogger` (`--log-file`), stamps `ts` when it writes a line.
- A failed delivery is logged and the run continues. The `GraphEventSink::publish` docs say a failed reliable event should stop the run at the next safe boundary, and that a `Dropped` disposition calls for a `Gap` event. Neither is implemented; the only production sink never returns an error.
- `resume_from` has no engine and so no sink. Replayed Activity nodes settle without a `NodeStarted`. Graph-level and wave events are still not published.

gap-8921a3 lists the GraphExecutionEvent sink as dead code; it is live now.

After the working branch at `aec267cac` was merged in (`5beb262c5`), a completed task node's outcome also covers gap-161be1's `TaskGateVerdict::PassedWithPreexistingFailures`, as `passed_with_preexisting_failures`.
