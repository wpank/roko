+++
id = "bug-9b69a3"
kind = "bug"
title = "Server-started plan runs publish to a private StateHub instead of the server's"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/serve-runtime", "roko-cli/graph-execution"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-cli/src/serve_runtime.rs::run_plan_on_local_runtime", "crates/roko-cli/src/graph_execution/plan_runner.rs:602"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = '! grep -q "shared_state_hub()" crates/roko-cli/src/graph_execution/plan_runner.rs'
+++

`run_plan_on_local_runtime` (`serve_runtime.rs:529`) receives serve's hub as `_state_hub` and drops it; the Graph plan runner then creates a fresh in-process hub with `crate::state_hub::shared_state_hub()` (`graph_execution/plan_runner.rs:602`).
A plan started through `POST /api/plans/{id}/execute` therefore shows only the `plan_started`/`plan_completed` events the route publishes itself; task, agent, gate and cost events never reach `/api/events`.
Fix: pass the caller's `SharedStateHub` into `run_graph_plan` (a field on `GraphPlanRunParams`) and use it instead of creating one.
Not covered by `plans/portal-programme/02-backend-plan-execution`; 03 T01-T05 only help server-started runs once this lands.
