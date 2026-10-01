+++
id = "bug-0a934f"
kind = "bug"
title = "Job runner writes job files without FileJobStore::save, leaving a stale legacy state key"
status = "done"
triage = "verified"
severity = "p3"
subsystem = ["roko-serve/jobs"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e6-jobs-tests"
anchors = ["crates/roko-serve/src/job_runner.rs::write_job", "crates/roko-core/src/job.rs::FileJobStore::save"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn write_job_clears_legacy_state_key' crates/roko-serve/ && cargo test -p roko-serve job_runner::tests::write_job_clears_legacy_state_key"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:08Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:13:45Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

`job_runner::write_job` serializes `MarketplaceJob` straight to disk. `FileJobStore::save` first folds the legacy `state` key into `status` and clears it (job.rs:1018-1030). When the runner executes a job that was created through the API with a `state` key, the file ends up with a fresh `status` and a stale `state`. Readers cope today because `status` wins (`effective_status`), but the file carries two status keys that disagree.

Fix: persist through `FileJobStore::save`, or apply the same folding in `write_job`.

## Notes

- 2026-10-01 (wk-serve2): implemented on work/bug-1cb461; cargo verification deferred to the batch check.
  `job_runner::write_job(workdir, job)` now persists through `FileJobStore::save` (atomic tmp + rename, legacy
  `state` folded into `status` and cleared). `execute_job` sets `job.id` to the id it was asked to run, so the save
  writes back to the file it read. Test `write_job_clears_legacy_state_key`.
