+++
id = "bug-0a934f"
kind = "bug"
title = "Job runner writes job files without FileJobStore::save, leaving a stale legacy state key"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-serve/jobs"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e6-jobs-tests"
anchors = ["crates/roko-serve/src/job_runner.rs::write_job", "crates/roko-core/src/job.rs::FileJobStore::save"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn write_job_clears_legacy_state_key' crates/roko-serve/ && cargo test -p roko-serve job_runner::tests::write_job_clears_legacy_state_key"
+++

`job_runner::write_job` serializes `MarketplaceJob` straight to disk. `FileJobStore::save` first folds the legacy `state` key into `status` and clears it (job.rs:1018-1030). When the runner executes a job that was created through the API with a `state` key, the file ends up with a fresh `status` and a stale `state`. Readers cope today because `status` wins (`effective_status`), but the file carries two status keys that disagree.

Fix: persist through `FileJobStore::save`, or apply the same folding in `write_job`.
