+++
id = "gap-b07969"
kind = "gap"
title = "execute {resume: true} cannot report skipped tasks at submit time — needs graph fingerprint before run"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-serve/plans", "roko-graph/engine"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "plan:portal-programme/03b-backend-workspace-server#T16"
discovered_from = "plan:portal-programme/03b-backend-workspace-server#T16"
anchors = ["crates/roko-serve/src/routes/plans.rs::execute_plan_endpoint", "crates/roko-graph/src/engine.rs::resume_or_start"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

`POST /api/plans/{id}/execute {resume: true}` returns 202 `{id, resume: true}`. The portal and CLI show `↻ Retry` as accepted, but neither the server response nor any early event tells the client which tasks will be skipped versus re-run.

**Root cause.** Knowing which tasks are skippable requires comparing the live checkpoint's graph fingerprint against the plan's current fingerprint. That comparison currently happens inside the Graph engine at run start — after the 202 has already been sent. Reading the checkpoint file before dispatching would give the answer, but the Graph engine's fingerprint logic is encapsulated in `crates/roko-graph/src/engine.rs` and is not exposed as a standalone query.

**Fix path.** Expose a `query_resume_plan(workdir, plan_id) -> ResumeSummary` function from the graph engine (or via `CliRuntime`) that loads the checkpoint and current plan, computes the fingerprint delta, and returns `{skippable_tasks: Vec<TaskId>, changed: bool}` without starting a run. The execute handler can call this before issuing the 202 and include `skippable_task_ids` in the response.
