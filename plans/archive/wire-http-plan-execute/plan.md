---
plan: wire-http-plan-execute
---

# Wire HTTP plan execute endpoint to the graph engine

`POST /api/plans/{id}/execute` in `crates/roko-serve/src/routes/plans.rs` currently
calls `runtime.run_once()` — a single LLM prompt invocation — instead of running the
plan through the graph engine. This means tasks defined in `tasks.toml` are never
individually dispatched, gates never run, and no per-task DashboardEvents are emitted.

The `CliRuntime` trait already has a `run_plan(workdir, plan_target)` method whose
concrete implementation in `crates/roko-cli/src/serve_runtime.rs` calls
`run_plan_on_local_runtime`, which delegates to the graph engine path. The `execute_plan`
handler ignores it and calls `run_once` directly instead.

The fix is two-step:

1. Replace the `runtime.run_once(&workdir, &prompt)` call inside `execute_plan` with
   `runtime.run_plan(&workdir, &plan_dir)` where `plan_dir` is the directory located by
   `find_plan`. The graph engine then handles task DAG execution, gate dispatch, and
   per-task checkpoint persistence.

2. After wiring the engine, forward the per-task `DashboardEvent`s (TaskStarted,
   TaskCompleted, GateResult) through the serve-side `event_bus` during execution so
   the portal SSE stream and TUI both see live progress without polling.

A reviewer then audits the execution flow end-to-end and writes a verdict document.
