+++
id = "bug-7e1b6b"
kind = "bug"
title = "The dashboard counts unverified and skipped tasks as passed"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["roko-cli/graph-tui-bridge", "roko-core/dashboard"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/cybernetic-harness/assessment-2026-09-28/spec-drift-from-commits.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/spec-drift-from-commits.md"
anchors = ["crates/roko-cli/src/runner/graph_tui_bridge.rs::node_outcome", "crates/roko-core/src/dashboard_snapshot.rs::classify_task_outcome"]
links = { depends_on = [], blocks = [], related = ["bug-6dc672"], supersedes = [], duplicate_of = "" }
+++
- `node_outcome` maps every completed node to "passed" whatever its verdict, including `Unverified`. Only `ForcedAccept` gets its own outcome (`graph_tui_bridge.rs`).
- `classify_task_outcome` (`dashboard_snapshot.rs:1107`) counts every outcome string without "fail" or "error" as passed, including "skipped" and "condition-skipped".
- Tasks accepted with failures count toward `plan.tasks_done`.

The TUI, SSE and portal therefore overstate verified success, and no dashboard count can be used as a success metric.

Fix: give unverified and skipped tasks their own outcomes end to end, and count only verified passes as passed.
