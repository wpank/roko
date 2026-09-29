+++
id = "bug-87ebdb"
kind = "bug"
title = "[refactor P1-04-build] SnapshotRebased variant left a non-exhaustive match in output_sink.rs (workspace build failed)"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/output_sink"]
created = 2026-09-15
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/refactoring-audit-2026-09-21/COMPLETION-STATUS.md#2026-09-15-session-batch-4"
discovered_from = "audit:tmp/archive/refactoring-audit-2026-09-21/COMPLETION-STATUS.md#2026-09-15-session-batch-4"
anchors = ["crates/roko-cli/src/runner/output_sink.rs:1493", "crates/roko-core/src/dashboard_snapshot.rs:439"]
links = { depends_on = [], blocks = [], related = ["gap-af73a8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -q "DashboardEvent::SnapshotRebased" crates/roko-cli/src/runner/output_sink.rs'

[closed]
at = 2026-09-28
commit = "91b4745f8"
evidence = "crates/roko-cli/src/runner/output_sink.rs:1493 matches `DashboardEvent::SnapshotRebased { .. } => return None` (arm present since 244f564e1); roko-core dashboard_snapshot.rs:2029 and roko-serve routes/status/health.rs:307 also handle the variant"
+++
P1-04 added DashboardEvent::SnapshotRebased; as a side effect the workspace build failed at output_sink.rs:1266 (non-exhaustive match) and the dogfood proof used a prebuilt binary. No fix recorded in the audit.

Imported without verification from:
- `tmp/archive/refactoring-audit-2026-09-21/COMPLETION-STATUS.md#2026-09-15-session-batch-4`

How to verify: grep SnapshotRebased in output_sink.rs and confirm the match arm exists; cargo check.

Verified 2026-09-28: fixed - output_sink.rs:1493 has the SnapshotRebased arm (since 244f564e1).
