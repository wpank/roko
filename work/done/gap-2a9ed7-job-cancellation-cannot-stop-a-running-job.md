+++
id = "gap-2a9ed7"
kind = "gap"
title = "Job cancellation cannot stop a running job: each cancel request builds its own JobExecutionService"
status = "done"
triage = "verified"
severity = "p2"
goal = "features"
subsystem = ["roko-serve/jobs"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e6-jobs-tests"
anchors = ["crates/roko-serve/src/routes/jobs.rs::cancel_job_endpoint", "crates/roko-serve/src/job_runner.rs::execute_job", "crates/roko-core/src/job.rs::JobExecutionService"]
links = { depends_on = [], blocks = [], related = ["bug-12be66"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'JobExecutionService::new(' crates/roko-serve/src/routes/jobs.rs && grep -qw 'fn cancel_running_auto_execute_job_stays_cancelled' crates/roko-serve/tests/job_runner_integration.rs && cargo test -p roko-serve --test job_runner_integration cancel_running_auto_execute_job_stays_cancelled"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:28Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:13:45Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

`cancel_job_endpoint` (routes/jobs.rs:1245) constructs a new `roko_core::JobExecutionService` for every request, so its map of cancel signals is always empty: the receipt reports `acknowledged: false` and nothing reaches the executor. `job_runner::execute_job` does not use the service at all. It watches only the server-wide cancel token, and it writes `submitted`/`completed`/`failed` from its in-memory copy of the job without re-reading the file, so a job cancelled while it runs can be overwritten by the runner's terminal write.

Fix: keep one `JobExecutionService` in `AppState`, shared by the route and the runner; have the runner observe per-job cancellation and never overwrite a terminal `cancelled` state. Add a job_runner integration test that cancels a running auto-execute job and asserts it stays cancelled.

Re-checked 2026-09-29: unchanged. The existing verify only checks that the per-request construction is gone; it would pass even if the runner still overwrote a cancelled job, so closure should also require the job_runner integration test this item asks for.

## Notes

- 2026-10-01 (wk-serve2): implemented on work/bug-1cb461; cargo verification deferred to the batch check.
  - `AppState.job_execution` holds one `JobExecutionService` (built in `AppState::new` for `.roko/jobs`);
    `cancel_job_endpoint` uses it instead of a per-request service.
  - `JobExecutionService::register_executor` / `unregister_executor` (`crates/roko-core/src/job.rs`) let an executor
    that drives the job itself (serve's runner holds its own lock file, so it cannot use `start`) receive
    `cancel`'s signal; `cancel` then reports `acknowledged: true`.
  - `job_runner::execute_job` registers after its `in_progress` write, runs the dispatch (now `dispatch_job`) in a
    `tokio::select!` against the cancel signal, and re-reads the file before any terminal write: a job found
    `cancelled` keeps that status, and the run returns an error without writing.
  - Tests: `job_execution_cancel_signals_registered_executor` (roko-core) and the integration test
    `cancel_running_auto_execute_job_stays_cancelled` (cancels a job whose runtime is blocked, then releases it).
  - Not changed: dropping the dispatch future does not stop work the CLI runtime runs on blocking threads
    (`run_plan`); the legacy `DELETE /api/jobs/{id}` still writes `cancelled` directly (the runner's re-read keeps
    it).
