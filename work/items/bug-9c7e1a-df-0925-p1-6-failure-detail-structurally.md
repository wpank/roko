+++
id = "bug-9c7e1a"
kind = "bug"
title = "DF-0925 P1-6: Failure detail structurally unavailable in the TUI"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["roko-core/dashboard"]
created = 2026-09-25
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P1-6. Failure detail is structurally unavailable"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P1-6. Failure detail is structurally unavailable"
anchors = ["crates/roko-core/src/dashboard_snapshot.rs::gate_failure_summary", "crates/roko-core/src/dashboard_snapshot.rs::FailureEntry", "crates/roko-cli/src/graph_task_dispatch.rs::published_gate_output", "crates/roko-cli/src/tui/widgets/error_digest.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-core failing_gate_summary_names_the_command_and_survives_other_tasks"

[closed]
at = 2026-09-28
commit = "725f21e05"
evidence = "Committed in 725f21e05 by the portal session. Re-checked 2026-09-28: the cited files are clean at HEAD and the named symbols and tests exist there (not rebuilt or retested here). All three defects are addressed: FailureEntry.summary is now gate_failure_summary(output) instead of String::new() (crates/roko-core/src/dashboard_snapshot.rs:1775); gate output is published with a leading `$ command` line (crates/roko-cli/src/graph_task_dispatch.rs published_gate_output); gate output is retained per task so a failure's detail outlives later gates (dashboard_snapshot.rs; test failing_gate_summary_names_the_command_and_survives_other_tasks). Reopen if not committed."
+++
FailureEntry.summary is hardcoded empty (and error_digest infers remediation from it); the failing verify command is never published (label only); gate_output_lines is cleared on every gate result.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P1-6. Failure detail is structurally unavailable`

How to verify: Fail a verify step; inspect Recent Failures panel.

Verified 2026-09-28: fixed in the uncommitted working tree (dashboard_snapshot.rs gate_failure_summary + per-task output retention; graph_task_dispatch.rs published_gate_output). Closure depends on that work being committed.
