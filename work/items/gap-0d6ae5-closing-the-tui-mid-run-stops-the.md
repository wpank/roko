+++
id = "gap-0d6ae5"
kind = "gap"
title = "Closing the TUI mid-run stops the plan run; there is no detach mode"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
subsystem = ["roko-cli/graph_execution", "roko-cli/tui"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e2-planset"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::pending_interrupt", "crates/roko-cli/src/graph_execution/plan_runner.rs::PlanRunInterrupt", "crates/roko-cli/src/serve_client.rs::follow_run_tui"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'stopping the plan run' crates/roko-cli/src/graph_execution/plan_runner.rs"
+++

Quitting the TUI while plans are still running is treated as an interrupt (`PlanRunInterrupt::Interrupt`, exit 130). Before the interrupt work, the run continued headless with stderr redirected and no terminal output. Neither suits an operator who closes the dashboard to look at something else.

Fix: offer a detach choice that keeps the run going and falls back to inline progress on the terminal (or to a hub the operator can reattach to).

Re-verified 2026-09-29 at d9e79e9d8: when a live roko-serve owns the workspace, roko plan run forwards the run to it (commands/plan.rs:565-575, 08a1fd272). Closing the follow TUI there does not cancel the run (serve_client.rs:723-741), so it acts as an implicit detach. What remains: local runs with no server still turn a TUI close into PlanRunInterrupt::Interrupt (plan_runner.rs:472-482), and neither path offers an explicit detach choice or falls back to inline terminal progress.
