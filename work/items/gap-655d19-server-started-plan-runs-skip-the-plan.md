+++
id = "gap-655d19"
kind = "gap"
title = "Server-started plan runs skip the PLAN_0xx validation that `roko plan run` enforces"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-serve/plans", "roko-cli/serve_runtime"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e3-discovery"
anchors = ["crates/roko-cli/src/commands/plan.rs::validate_before_run", "crates/roko-cli/src/serve_runtime.rs::run_plan_on_local_runtime", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -rqE "validate_plans_dir|validate_before_run" crates/roko-cli/src/serve_runtime.rs crates/roko-cli/src/graph_execution/plan_runner.rs'
+++

The CLI validates before it runs: `commands/plan.rs::validate_before_run` calls `plan_validate::validate_plans_dir_with_workdir` and refuses on errors. `run_graph_plan`, which serve's `run_plan_on_local_runtime` calls for `POST /api/plans/{id}/execute` and the plan-set route, only parses plans through `runner::plan_loader::load_plans`. Rules such as missing prerequisites (PLAN_031), write capability (PLAN_036) and context ranges (PLAN_CONTEXT_*) are therefore enforced for CLI runs but not for runs started from the portal. The authoring routes (plan 04) validate drafts through `plan_authoring::validate_plan_source`; execute does not.

Fix: run the same validation inside `run_graph_plan` (or at execute admission) and return the diagnostics as a 422.
