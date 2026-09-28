+++
id = "bug-9b69a3"
kind = "bug"
title = "Server-started plan runs publish to a private StateHub instead of the server's"
status = "done"
triage = "verified"
severity = "p1"
goal = "visibility"
subsystem = ["roko-cli/serve-runtime", "roko-cli/graph-execution"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-cli/src/serve_runtime.rs::run_plan_on_local_runtime", "crates/roko-cli/src/graph_execution/plan_runner.rs:602", "crates/roko-cli/src/graph_execution/plan_runner.rs:956"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = '! grep -q "shared_state_hub()" crates/roko-cli/src/graph_execution/plan_runner.rs'

[closed]
at = 2026-09-28
commit = "725f21e05"
by = "triage check 2026-09-28"
evidence = "run_plan_on_local_runtime now takes `state_hub: SharedStateHub` (serve_runtime.rs:529-534) and passes `state_hub: Some(state_hub)` into GraphPlanRunParams (:582, 'Publish into the server's hub'); GraphPlanRunParams has `pub state_hub: Option<SharedStateHub>` (graph_execution/plan_runner.rs:618) and run_graph_plan uses `state_hub.unwrap_or_else(shared_state_hub)` (:956), covered by test provided_state_hub_receives_the_runs_plan_events (:2190); commands/server.rs:33-50 builds the runtime and AppState from the same state_hub_for_workdir hub. Note the item's [[repro]] grep for `shared_state_hub()` still matches the test helpers at plan_runner.rs:2214/:2308, so it cannot tell fixed from broken. (Static check against 3d0ee4d02; tests not re-run.)"
+++

`run_plan_on_local_runtime` (`serve_runtime.rs:529`) receives serve's hub as `_state_hub` and drops it; the Graph plan runner then creates a fresh in-process hub with `crate::state_hub::shared_state_hub()` (`graph_execution/plan_runner.rs:602`).
A plan started through `POST /api/plans/{id}/execute` therefore shows only the `plan_started`/`plan_completed` events the route publishes itself; task, agent, gate and cost events never reach `/api/events`.
Fix: pass the caller's `SharedStateHub` into `run_graph_plan` (a field on `GraphPlanRunParams`) and use it instead of creating one.
Not covered by `plans/portal-programme/02-backend-plan-execution`; 03 T01-T05 only help server-started runs once this lands.

Fixed in 725f21e05 (checked 2026-09-28).
