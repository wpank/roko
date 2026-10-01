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
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:w3a-tui-polish"
anchors = ["crates/roko-core/src/dashboard_snapshot.rs::retain_task_gate_output"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "sed -n '/^fn retain_task_gate_output/,/^}/p' crates/roko-core/src/dashboard_snapshot.rs | grep -q 'entry.gate != gate'"
+++

`retain_task_gate_output` drops any earlier entry for the same plan and task before storing a new one (dashboard_snapshot.rs:3616-3618). A task with several verify steps, or a failed step followed by a passing retry, keeps only the last step's output, so the TUI gate panel shows earlier steps as pass/fail without their command or output.

Fix: retain output per task and step (bounded per task) and render the step list.

## Notes

- 2026-10-01 (wk-tuiv): implemented on work/bug-6c11d1; cargo verification deferred to the batch check.
  `retain_task_gate_output` keys retained output by plan, task and gate: a rerun replaces only that step's output,
  with at most `MAX_TASK_GATE_STEPS` (8) steps per task and `MAX_TASK_GATE_OUTPUTS` raised from 64 to 128 entries
  overall. `DashboardSnapshot::task_gate_step_output` looks up one step; failure summaries use it. The TUI Verify
  sub-view shows each step's command on its row and the output tail of the latest failing step (else the latest
  step). Tests: `task_gate_outputs_keep_each_verify_step` (roko-core) and
  `verify_tab_lists_each_steps_command_and_the_failing_output` (roko-cli). The portal already keys outputs by
  plan, task and gate, so it now gets every step's output with no change.
