+++
id = "bug-0fd558"
kind = "bug"
title = "DF-0926 N-8: Job cancel returns 500 (JobError mapped to internal); tests ignored"
status = "superseded"
triage = "verified"
severity = "p2"
subsystem = ["roko-serve/routes/jobs"]
created = 2026-09-26
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#N-8. Whole-crate gates inherit pre-existing failures"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#N-8. Whole-crate gates inherit pre-existing failures"
anchors = ["routes/jobs.rs:1217", "JobExecutionService::cancel", "crates/roko-serve/tests/job_lifecycle.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "bug-12be66" }

[closed]
at = 2026-09-28
evidence = "duplicate of bug-12be66 (verified 2026-09-28): same defect, same handler (cancel_job_endpoint, routes/jobs.rs:1217) and the same three ignored job_lifecycle tests; bug-12be66 carries the repro and verify commands."
+++
POST /api/jobs/{id}/cancel returns 500 because every JobError except InvalidTransition maps to ApiError::internal, masking JobExecutionService::cancel failures; three job_lifecycle tests marked #[ignore] and logged to GAPS.md.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#N-8. Whole-crate gates inherit pre-existing failures`
- `tmp/dogfood/2026-09-25-portal-programme-run.md#Unresolved`

How to verify: Run job_lifecycle tests with --ignored.
