+++
id = "bug-4f833d"
kind = "bug"
title = "POST /api/plans/{id}/execute returns a run id the Graph engine never uses"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["roko-serve/plans", "roko-cli/serve-runtime"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/routes/plans.rs::start_plan_run", "crates/roko-serve/src/routes/plans.rs::execute_plans", "crates/roko-serve/src/runtime.rs::PlanRunOptions", "crates/roko-cli/src/serve_runtime.rs::run_plan_with_options", "crates/roko-cli/src/graph_execution/plan_runner.rs:2002"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'cargo test -p roko-serve --test plan_execute'
+++

The execute handler mints `run_id = Uuid::new_v4()` (`routes/plans.rs:266`) and returns it in the 202, but calls `runtime.run_plan(&workdir, &plan_dir)` (`:301`) without it.
The Graph engine mints its own run id for the checkpoint and no route exposes it, so callers cannot correlate the 202 with the checkpoint, events, costs or a later resume.
Fix: thread a caller-visible run id (with cancel and state-hub options) through a widened `CliRuntime` run call into checkpoint creation and return it in the 202 (keep `id` for compatibility); assert it in `tests/plan_execute.rs`.
Not covered by the current `plans/portal-programme/02-backend-plan-execution` tasks.

Re-verified 2026-09-29 at d9e79e9d8: still open. Plans 03/03b/03c moved the handler into start_plan_run and threaded a CancelToken, fresh/force_resume and live output through PlanRunOptions (crates/roko-serve/src/runtime.rs:83), so the cancel part of the proposed fix exists, but PlanRunOptions carries no run id: the 202 id (routes/plans.rs:475) and the plan-set id (routes/plans.rs:381) are still never passed to the engine, which takes its run id from the checkpoint (roko-cli graph_execution/plan_runner.rs:2002). tests/plan_execute.rs only asserts the 202 id is non-empty.

The [[verify]] command is unsound (see the check notes). Proposed replacement, not yet validated: `grep -q 'pub run_id' crates/roko-serve/src/runtime.rs && cargo test -p roko-serve --test plan_execute execute_passes_run_id_to_runtime (test the fix must add: stub runtime records PlanRunOptions and asserts its run id equals the 202 id; the existing plan_execute tests only check the id is non-empty and pass today)`.
