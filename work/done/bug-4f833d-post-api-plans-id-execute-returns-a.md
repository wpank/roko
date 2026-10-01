+++
id = "bug-4f833d"
kind = "bug"
title = "POST /api/plans/{id}/execute returns a run id the Graph engine never uses"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["roko-serve/plans", "roko-cli/serve-runtime"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/routes/plans.rs::start_plan_run", "crates/roko-serve/src/routes/plans.rs::execute_plans", "crates/roko-serve/src/runtime.rs::PlanRunOptions", "crates/roko-cli/src/serve_runtime.rs::run_plan_with_options", "crates/roko-cli/src/graph_execution/plan_runner.rs:2002"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'pub run_id: Option<String>' crates/roko-serve/src/runtime.rs && cargo test -p roko-serve --test plan_execute execute_passes_run_id_to_runtime"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:16Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:46Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

The execute handler mints `run_id = Uuid::new_v4()` (`routes/plans.rs:266`) and returns it in the 202, but calls `runtime.run_plan(&workdir, &plan_dir)` (`:301`) without it.
The Graph engine mints its own run id for the checkpoint and no route exposes it, so callers cannot correlate the 202 with the checkpoint, events, costs or a later resume.
Fix: thread a caller-visible run id (with cancel and state-hub options) through a widened `CliRuntime` run call into checkpoint creation and return it in the 202 (keep `id` for compatibility); assert it in `tests/plan_execute.rs`.
Not covered by the current `plans/portal-programme/02-backend-plan-execution` tasks.

Re-verified 2026-09-29 at d9e79e9d8: still open. Plans 03/03b/03c moved the handler into start_plan_run and threaded a CancelToken, fresh/force_resume and live output through PlanRunOptions (crates/roko-serve/src/runtime.rs:83), so the cancel part of the proposed fix exists, but PlanRunOptions carries no run id: the 202 id (routes/plans.rs:475) and the plan-set id (routes/plans.rs:381) are still never passed to the engine, which takes its run id from the checkpoint (roko-cli graph_execution/plan_runner.rs:2002). tests/plan_execute.rs only asserts the 202 id is non-empty.

The [[verify]] command is unsound (see the check notes). Proposed replacement, not yet validated: `grep -q 'pub run_id' crates/roko-serve/src/runtime.rs && cargo test -p roko-serve --test plan_execute execute_passes_run_id_to_runtime (test the fix must add: stub runtime records PlanRunOptions and asserts its run id equals the 202 id; the existing plan_execute tests only check the id is non-empty and pass today)`.

## Notes

2026-10-01 (wk-runstate): implemented on work/find-8872ad; cargo verification deferred to the batch check. `PlanRunOptions` has a `run_id`. `POST /api/plans/{id}/execute`, `POST /api/plans/{id}/resume` and `POST /api/plans/execute` pass it the id their 202 returns; the 202 now also carries that id as `run_id`, and `id` is unchanged. The CLI runtime runs the plan through `run_graph_plan_in_run` with that id. So the run's events and per-run event index, its `status.json` and, for a fresh single plan, its checkpoint and `.roko/runs/<id>/` attempt records all take the id. Under `run_graph_plan_in_run`'s rules, a resumed plan's checkpoint keeps the run it resumes, and each plan of a multi-plan set mints its own checkpoint run. The test `execute_passes_run_id_to_runtime` (`tests/plan_execute.rs`) checks both routes against a recording stub. The [[verify]] is now the command proposed above, with the grep narrowed to `pub run_id: Option<String>` (`SweBenchRunResult` already had a `pub run_id`); the old one passed whatever the code did.
