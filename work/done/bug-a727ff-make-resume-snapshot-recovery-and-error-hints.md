+++
id = "bug-a727ff"
kind = "bug"
title = "Make Resume Snapshot Recovery and Error Hints Truthful"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["roko-gate"]
created = 2026-09-07
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/326-resume-snapshot-recovery-error-hints.md#326 — Make Resume Snapshot Recovery and Error Hints Truthful"
discovered_from = "audit:tmp/backlog/archive/326-resume-snapshot-recovery-error-hints.md#326 — Make Resume Snapshot Recovery and Error Hints Truthful"
anchors = ["crates/roko-runtime/src/state_snapshot.rs::validate_state_snapshot", "crates/roko-cli/src/runner/resume.rs:141", "crates/roko-cli/src/main.rs::error_hint_authoritative_with_state_recovery"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli error_hint_authoritative_with_state_recovery"

[[verify]]
command = "cargo test -p roko-runtime state_snapshot"

[closed]
at = 2026-09-28
commit = "91b4745f8"
evidence = "Implemented 2026-09-04 per the source packet and present at 91b4745f8: snapshots are validated by roko_runtime::validate_state_snapshot (crates/roko-runtime/src/state_snapshot.rs:306), failures surface as StateRecoveryRequired (crates/roko-cli/src/runner/resume.rs:141), and error_hint maps state-recovery messages to the recovery hint rather than the API-key hint (main.rs test error_hint_authoritative_with_state_recovery). The Runner-v2 engine that wrote the inconsistent lifecycle has since been removed (--engine legacy errors, crates/roko-cli/src/main.rs:2013)."
+++
a completed plan can poison the next `plan run`, while the CLI recommends an unrelated credential fix. A successful one-task Runner-v2 plan wrote `.roko/state/state-snapshot.json` with `tasks_total = 1` and `lifecycle.total_tasks = 1`, but `lifecycle.tasks` contained two entries: the real `T01`…

Imported without verification from:
- `tmp/backlog/archive/326-resume-snapshot-recovery-error-hints.md#326 — Make Resume Snapshot Recovery and Error Hints Truthful`

Warning: every file this item cites is gone (`.roko/state/state-snapshot.json`) — likely obsolete or moved.

How to verify: Check: A completed one-task plan with plan verification cannot persist a lifecycle where; Snapshots emitted during plan-verify start, success, failure, and regeneration all pass the; When the authoritative snapshot is invalid and no backup is… [evidence: own status: Implemented (2026-09-04); 00-STATUS-SUMMARY 3. Open / P1 -- High (Open): S | 3 | Done (2026-09-04); (newer evidence overrides own status "closed")]

Verified 2026-09-28: closed as done; validation and truthful hint are in code, and the Runner-v2 writer is gone.
