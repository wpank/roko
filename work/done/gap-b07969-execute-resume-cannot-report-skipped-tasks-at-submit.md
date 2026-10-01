+++
id = "gap-b07969"
kind = "gap"
title = "execute {resume: true} cannot report skipped tasks at submit time — needs graph fingerprint before run"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
subsystem = ["roko-serve/plans", "roko-graph/engine"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "plan:portal-programme/03b-backend-workspace-server#T16"
discovered_from = "plan:portal-programme/03b-backend-workspace-server#T16"
anchors = ["crates/roko-serve/src/routes/plans.rs::execute_plan", "crates/roko-serve/src/routes/plans.rs::start_plan_run", "crates/roko-graph/src/fingerprint.rs::plan_graph_fingerprint", "crates/roko-cli/src/graph_checkpoint.rs:360", "crates/roko-graph/src/engine.rs::resume_from"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'skippable_task_ids' crates/roko-serve/src/routes/plans.rs && grep -rqw 'fn execute_resume_reports_skippable_tasks' crates/roko-serve/src && cargo test -p roko-serve --lib execute_resume_reports_skippable_tasks"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:46Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:54Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

`POST /api/plans/{id}/execute {resume: true}` returns 202 `{id, resume: true}`. The portal and CLI show `↻ Retry` as accepted, but neither the server response nor any early event tells the client which tasks will be skipped versus re-run.

**Root cause.** Knowing which tasks are skippable requires comparing the live checkpoint's graph fingerprint against the plan's current fingerprint. That comparison currently happens inside the Graph engine at run start — after the 202 has already been sent. Reading the checkpoint file before dispatching would give the answer, but the Graph engine's fingerprint logic is encapsulated in `crates/roko-graph/src/engine.rs` and is not exposed as a standalone query.

**Fix path.** Expose a `query_resume_plan(workdir, plan_id) -> ResumeSummary` function from the graph engine (or via `CliRuntime`) that loads the checkpoint and current plan, computes the fingerprint delta, and returns `{skippable_tasks: Vec<TaskId>, changed: bool}` without starting a run. The execute handler can call this before issuing the 202 and include `skippable_task_ids` in the response.

Re-checked 2026-09-29: unchanged; POST /api/plans/{id}/execute still returns only {id, resume} (crates/roko-serve/src/routes/plans.rs:594-597). The anchor roko-graph engine.rs::resume_or_start does not exist. The building blocks for a preview now exist: roko_graph::plan_graph_fingerprint (crates/roko-graph/src/fingerprint.rs:141) and the checkpoint fingerprint computation in crates/roko-cli/src/graph_checkpoint.rs:360-364, so a fix can compare the stored checkpoint fingerprint with the current plan's before sending the 202.

## Notes

- 2026-10-01 (wk-tamper): implemented on work/gap-7147bb; cargo verification deferred to the batch check.
  `CliRuntime::resume_skippable_tasks` (default `Ok(None)`) reads what a resume would replay. The CLI runtime answers
  with `graph_checkpoint::preview_plan_resume` under the server's resume options (`force_resume`).
  `start_plan_run` asks before it spawns the run, and `POST /api/plans/{id}/execute` and `POST /api/plans/{id}/resume`
  return `skippable_task_ids` in their 202: `[]` for a fresh run, `null` when the runtime cannot tell. A failed preview
  only logs a warning. The `[[verify]]` was not a runnable command; it now greps and runs the new test. The portal's
  `WireAccepted` type does not read the field yet.
