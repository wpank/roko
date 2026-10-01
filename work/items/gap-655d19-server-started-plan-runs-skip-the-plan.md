+++
id = "gap-655d19"
kind = "gap"
title = "Server-started plan runs skip the PLAN_0xx validation that `roko plan run` enforces"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-serve/plans", "roko-cli/serve_runtime"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e3-discovery"
anchors = ["crates/roko-cli/src/commands/plan.rs::validate_before_run", "crates/roko-cli/src/serve_runtime.rs::plan_run_order", "crates/roko-cli/src/serve_runtime.rs::run_plan_on_local_runtime", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan", "crates/roko-serve/src/routes/plans.rs::execute_plans"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'validate_plans_dir_with_workdir' crates/roko-cli/src/serve_runtime.rs && cargo test -p roko-cli --lib serve_runtime::tests::server_plan_runs_are_validated_like_roko_plan_run && cargo test -p roko-serve --test plan_execute execute_refuses_a_plan_that_fails_validation"
+++

The CLI validates before it runs: `commands/plan.rs::validate_before_run` calls `plan_validate::validate_plans_dir_with_workdir` and refuses on errors. `run_graph_plan`, which serve's `run_plan_on_local_runtime` calls for `POST /api/plans/{id}/execute` and the plan-set route, only parses plans through `runner::plan_loader::load_plans`. Rules such as missing prerequisites (PLAN_031), write capability (PLAN_036) and context ranges (PLAN_CONTEXT_*) are therefore enforced for CLI runs but not for runs started from the portal. The authoring routes (plan 04) validate drafts through `plan_authoring::validate_plan_source`; execute does not.

Fix: run the same validation inside `run_graph_plan` (or at execute admission) and return the diagnostics as a 422.

Re-checked 2026-09-29: unchanged. The serve execute path is routes/plans.rs execute_plans/execute_plan -> serve_runtime.rs plan_run_order (load_plans only) -> run_plan_on_local_runtime (:815) -> run_graph_plan; none of them call plan_validate.

## Notes

2026-10-01 (wk-runstate): implemented on work/find-8872ad; cargo verification deferred to the batch check. `CliRuntime` has a new method, `validate_plan_run`. Its default admits every run, so stubs and remote runtimes are unchanged. `RokoCliRuntime` implements it as `plan_run_validation`, the check `roko plan run` makes in `validate_before_run`: `plan_validate::validate_plans_dir_with_workdir` with the workspace's models, where an error that is not advisory stops the run. The check covers only the plans the run names, matched by plan id or directory. `start_plan_run` (execute and resume) and `execute_plans` call it before they take the run lock, and they refuse with a 422 whose `details` holds the validation report (`plan_run_rejected`). Tests: `server_plan_runs_are_validated_like_roko_plan_run` (serve_runtime.rs, against a real plan with a PLAN_005 error) and `execute_refuses_a_plan_that_fails_validation` (`roko-serve` `tests/plan_execute.rs`, both routes, no run started). The [[verify]] now runs these tests. The old one was a grep, which a comment could satisfy. `docs/v3/26-HTTP-API.md` notes the 422.
