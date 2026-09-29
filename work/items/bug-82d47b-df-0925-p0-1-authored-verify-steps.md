+++
id = "bug-82d47b"
kind = "bug"
title = "DF-0925 P0-1: Authored verify steps are force-accepted after the review-cycle cap"
status = "done"
triage = "verified"
severity = "p0"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-25
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P0-1. Authored verify steps are non-blocking by construction"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P0-1. Authored verify steps are non-blocking by construction"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::settle_task_verification", "crates/roko-cli/src/graph_checkpoint.rs::GATE_VERDICT_EXTENSION", "crates/roko-cli/src/graph_checkpoint.rs::invalidate_unverified_activities", "crates/roko-graph/src/cells/task_executor.rs::TaskGateVerdict"]
links = { depends_on = [], blocks = [], related = ["bug-06e2d1", "bug-6dc672"], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'cargo test -p roko-cli --lib resume_reruns_unverified_or_forced_task_records'

[[verify]]
command = '! grep -n "max_review_cycles" crates/roko-cli/src/graph_task_dispatch.rs | grep -v "//"'

[closed]
at = 2026-09-28
commit = "725f21e05"
evidence = "Committed in 725f21e05 by the portal session. Re-checked 2026-09-28: the cited files are clean at HEAD and the named symbols and tests exist there (not rebuilt or retested here). crates/roko-cli/src/graph_task_dispatch.rs:1402-1410, 1946-1951 and 3195-3198: failing authored verify steps return RokoError::Verify and are never force-accepted (no max_review_cycles logic remains); graph_checkpoint.rs:271 writes the GATE_VERDICT_EXTENSION summary and graph_checkpoint.rs:225 invalidate_unverified_activities re-executes forced/unverified records on resume"
+++
When cycle_count >= max_review_cycles the dispatcher force-accepts and logs 'all graph verify steps passed'; with default max_retries=3 the last attempt is always force-accepted, so verify can never fail a task. No event/field/checkpoint trace; GATE_VERDICT_EXTENSION unused.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P0-1. Authored verify steps are non-blocking by construction`
- `tmp/dogfood/2026-09-25-portal-programme-run.md#Architecture recommendations`

How to verify: Run a task whose verify always fails with default config; check final status and logs.

Verified 2026-09-28: fixed in working tree - graph_task_dispatch.rs:1946 (verify failure fails the attempt), graph_checkpoint.rs:47/271 (verdict extension written). crates/roko-graph/src/cells/task_executor.rs still exists; the import warning was wrong.
