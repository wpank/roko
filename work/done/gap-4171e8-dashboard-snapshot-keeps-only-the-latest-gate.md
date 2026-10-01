+++
id = "gap-4171e8"
kind = "gap"
title = "Dashboard snapshot keeps only the latest gate output per task, so earlier verify steps lose their output"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
subsystem = ["roko-core/dashboard_snapshot", "roko-cli/tui"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:w3a-tui-polish"
anchors = ["crates/roko-core/src/dashboard_snapshot.rs::retain_task_gate_output"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "sed -n '/^fn retain_task_gate_output/,/^}/p' crates/roko-core/src/dashboard_snapshot.rs | grep -q 'entry.gate != gate'"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:37Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:13:11Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
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
