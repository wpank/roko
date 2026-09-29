+++
id = "gap-2a9ed7"
kind = "gap"
title = "Job cancellation cannot stop a running job: each cancel request builds its own JobExecutionService"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-serve/jobs"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e6-jobs-tests"
anchors = ["crates/roko-serve/src/routes/jobs.rs::cancel_job_endpoint", "crates/roko-serve/src/job_runner.rs::execute_job", "crates/roko-core/src/job.rs:562"]
links = { depends_on = [], blocks = [], related = ["bug-12be66"], supersedes = [], duplicate_of = "" }

[[verify]]
command = '! grep -q "JobExecutionService::new(jobs_dir" crates/roko-serve/src/routes/jobs.rs'
+++

`cancel_job_endpoint` (routes/jobs.rs:1245) constructs a new `roko_core::JobExecutionService` for every request, so its map of cancel signals is always empty: the receipt reports `acknowledged: false` and nothing reaches the executor. `job_runner::execute_job` does not use the service at all. It watches only the server-wide cancel token, and it writes `submitted`/`completed`/`failed` from its in-memory copy of the job without re-reading the file, so a job cancelled while it runs can be overwritten by the runner's terminal write.

Fix: keep one `JobExecutionService` in `AppState`, shared by the route and the runner; have the runner observe per-job cancellation and never overwrite a terminal `cancelled` state. Add a job_runner integration test that cancels a running auto-execute job and asserts it stays cancelled.
