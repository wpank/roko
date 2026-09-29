+++
id = "gap-4171e8"
kind = "gap"
title = "Dashboard snapshot keeps only the latest gate output per task, so earlier verify steps lose their output"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
subsystem = ["roko-core/dashboard_snapshot", "roko-cli/tui"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:w3a-tui-polish"
anchors = ["crates/roko-core/src/dashboard_snapshot.rs::retain_task_gate_output"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "sed -n '/^fn retain_task_gate_output/,/^}/p' crates/roko-core/src/dashboard_snapshot.rs | grep -q 'entry.gate != gate'"
+++

`retain_task_gate_output` drops any earlier entry for the same plan and task before storing a new one (dashboard_snapshot.rs:3616-3618). A task with several verify steps, or a failed step followed by a passing retry, keeps only the last step's output, so the TUI gate panel shows earlier steps as pass/fail without their command or output.

Fix: retain output per task and step (bounded per task) and render the step list.
