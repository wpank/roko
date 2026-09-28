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
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/routes/plans.rs:266", "crates/roko-serve/src/routes/plans.rs:301", "crates/roko-serve/src/runtime.rs::CliRuntime"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'cargo test -p roko-serve --test plan_execute'
+++

The execute handler mints `run_id = Uuid::new_v4()` (`routes/plans.rs:266`) and returns it in the 202, but calls `runtime.run_plan(&workdir, &plan_dir)` (`:301`) without it.
The Graph engine mints its own run id for the checkpoint and no route exposes it, so callers cannot correlate the 202 with the checkpoint, events, costs or a later resume.
Fix: thread a caller-visible run id (with cancel and state-hub options) through a widened `CliRuntime` run call into checkpoint creation and return it in the 202 (keep `id` for compatibility); assert it in `tests/plan_execute.rs`.
Not covered by the current `plans/portal-programme/02-backend-plan-execution` tasks.
