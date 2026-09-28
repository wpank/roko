+++
id = "bug-0fd558"
kind = "bug"
title = "DF-0926 N-8: Job cancel returns 500 (JobError mapped to internal); tests ignored"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-serve/routes/jobs"]
created = 2026-09-26
updated = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#N-8. Whole-crate gates inherit pre-existing failures"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#N-8. Whole-crate gates inherit pre-existing failures"
anchors = ["routes/jobs.rs:1217", "JobExecutionService::cancel", "crates/roko-serve/tests/job_lifecycle.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
POST /api/jobs/{id}/cancel returns 500 because every JobError except InvalidTransition maps to ApiError::internal, masking JobExecutionService::cancel failures; three job_lifecycle tests marked #[ignore] and logged to GAPS.md.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#N-8. Whole-crate gates inherit pre-existing failures`
- `tmp/dogfood/2026-09-25-portal-programme-run.md#Unresolved`

How to verify: Run job_lifecycle tests with --ignored.
