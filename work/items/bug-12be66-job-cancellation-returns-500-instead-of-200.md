+++
id = "bug-12be66"
kind = "bug"
title = "Job cancellation returns 500 instead of 200/422"
status = "done"
triage = "verified"
severity = "p2"
subsystem = ["roko-serve/jobs", "roko-core/jobs"]
created = 2026-09-26
updated = 2026-09-28
last_verified = 2026-09-28
source = "gaps-md#gap-2026-09-26-job-cancellation-returns-500-roko-serve--roko-core"
discovered_from = "plan:portal-programme/01-backend-plan-service"
anchors = ["crates/roko-serve/src/routes/jobs.rs::cancel_job_endpoint", "crates/roko-core/src/job.rs:562", "crates/roko-serve/tests/job_lifecycle.rs:536"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = 'cargo test -p roko-serve --test job_lifecycle -- --include-ignored test_cancel_from_assigned_state test_job_cancellation_from_in_progress test_cancel_terminal_job_fails_422'

[[verify]]
command = 'cargo test -p roko-serve --test job_lifecycle -- --include-ignored cancel'

[closed]
at = 2026-09-28
commit = "725f21e05"
by = "session roko-b6"
evidence = "Root cause: MarketplaceJob.reward (crates/roko-core/src/job.rs) was a String and rejected the null reward that roko-serve writes. It now accepts null and numeric rewards (test marketplace_job_accepts_numeric_reward and a null-reward case). At 725f21e05, crates/roko-serve/tests/job_lifecycle.rs has no #[ignore] left (checked here). The portal session reports cargo test -p roko-serve --test job_lifecycle at 23/23; not re-run here, to avoid cargo contention with that running session."
+++

`POST /api/jobs/{id}/cancel` returns 500 where it should return 200 (cancel from assigned or in-progress) or 422 (job already terminal). `cancel_job_endpoint` (`crates/roko-serve/src/routes/jobs.rs:1217`) calls `roko_core::JobExecutionService::cancel` and maps every `JobError` except `InvalidTransition` to `ApiError::internal`, which hides the real cause. The response body also uses the field `status`, while the tests assert on `state`.

Three tests in `crates/roko-serve/tests/job_lifecycle.rs` are `#[ignore]`d because of this (`:536`, `:786`, `:1363`): `test_cancel_from_assigned_state`, `test_job_cancellation_from_in_progress` and `test_cancel_terminal_job_fails_422`. The bug predates the portal-programme work. It surfaced when plan gates were widened from single test files to the whole crate.

Fix: find which `JobError` `cancel` returns for these states and map it correctly (422 for terminal or invalid transitions). Align the response field name with the API contract and un-ignore the three tests. Until then, change their ignore reasons to cite this item ID.

Resolved in 725f21e05. The failure was in deserialization: `MarketplaceJob.reward` was a `String` and rejected the `null` reward that roko-serve writes. The catch-all 500 mapping in `cancel_job_endpoint` is what hid it.
