+++
id = "bug-5c2d01"
kind = "bug"
title = "DF-0925 P3-1: SIGTERM leaves the checkpoint 'running' and exits EXIT_SUCCESS"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/main"]
created = 2026-09-25
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P3-1. SIGTERM never finalizes the checkpoint, and exits EXIT_SUCCESS"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P3-1. SIGTERM never finalizes the checkpoint, and exits EXIT_SUCCESS"
anchors = ["crates/roko-cli/src/main.rs::install_sigterm_handler", "crates/roko-cli/src/graph_execution/plan_runner.rs::PlanRunInterrupt", "crates/roko-cli/src/graph_checkpoint.rs::GraphCheckpointStatus"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'PlanRunInterrupt::Terminate.exit_code()' crates/roko-cli/src/main.rs"

[closed]
at = 2026-09-28
commit = "91b4745f8"
evidence = "(working tree, uncommitted; not in HEAD 91b4745f8) crates/roko-cli/src/main.rs::install_sigterm_handler defers to a running plan run (plan_run_owns_termination_signals) and otherwise exits PlanRunInterrupt::Terminate.exit_code() (143), never EXIT_SUCCESS; plan_runner.rs finalizes checkpoints as GraphCheckpointStatus::Interrupted/Cancelled (graph_checkpoint.rs finish_with_status)."
+++
The signal handler reaps children then calls process::exit(EXIT_SUCCESS); PreparedGraphCheckpoint has no Drop, so an interrupted run reports success to CI and the checkpoint is never finalized.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P3-1. SIGTERM never finalizes the checkpoint, and exits EXIT_SUCCESS`

How to verify: SIGTERM a plan run; check exit code and checkpoint status.

Verified 2026-09-28: closed as done; see [closed].evidence.
