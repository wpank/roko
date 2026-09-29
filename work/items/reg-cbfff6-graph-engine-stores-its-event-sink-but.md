+++
id = "reg-cbfff6"
kind = "regression"
title = "Graph engine stores its event sink but never emits to it"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["roko-graph/engine", "roko-cli/state-hub"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "gaps-md#batch-2026-09-05-engine-convergence/statehubgrapheventsink"
discovered_from = "doc:tmp/work-management/01-gaps-md-audit.md"
anchors = ["crates/roko-graph/src/engine.rs::with_event_sink", "crates/roko-graph/src/engine.rs:327", "crates/roko-cli/src/graph_execution/plan_runner.rs:2016", "crates/roko-cli/src/runner/types.rs:2662", "crates/roko-cli/src/runner/types.rs:2717"]
links = { depends_on = [], blocks = [], related = ["gap-7f5eb6", "gap-d40bc0", "gap-8a1fb3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = '''grep -n 'event_sink' crates/roko-graph/src/engine.rs | grep -vE 'event_sink: |fn with_event_sink|self\.event_sink = Some' | grep -q .'''
+++

The 2026-09-05 engine-convergence batch recorded `StateHubGraphEventSink` as "wired into all graph execution paths". `GraphEngine` stores the sink (`crates/roko-graph/src/engine.rs:323`, set by `with_event_sink` at `:444`), but no engine code reads it, so the engine itself emits no `GraphExecutionEvent`s. Dashboards only get what the CLI host layer bridges separately, and they get it in batches (see the related items). `RunConfig.http_event_sink` is also always `None` (`crates/roko-cli/src/runner/types.rs:2662`, `:2717`; `crates/roko-cli/src/commands/do_cmd.rs:949`). The CLI-to-serve hub bridge is tracked in gap-7f5eb6.

Fix: have the engine emit node lifecycle events (start, complete, fail, skip, with timestamps) through the stored sink. Add a test in which a sink attached with `with_event_sink` receives them during a real run.

Verified 2026-09-28 (static check against 3d0ee4d02): GraphEngine still only stores the sink: `event_sink` appears in roko-graph solely as the field (engine.rs:327), its None initializer (:353) and the with_event_sink setter (:448-449); nothing in crates/roko-graph/src reads or emits through it (the item's [[verify]] grep still finds no other use). RunConfig.http_event_sink is still only ever None (runner/types.rs:2662, :2717); the do_cmd.rs:949 anchor is stale because do_cmd.rs no longer references RunConfig/http_event_sink.
