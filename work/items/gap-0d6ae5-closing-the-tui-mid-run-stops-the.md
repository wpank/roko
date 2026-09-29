+++
id = "gap-0d6ae5"
kind = "gap"
title = "Closing the TUI mid-run stops the plan run; there is no detach mode"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-cli/graph_execution", "roko-cli/tui"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e2-planset"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::pending_interrupt", "crates/roko-cli/src/graph_execution/plan_runner.rs::PlanRunInterrupt"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -q "detach" crates/roko-cli/src/graph_execution/plan_runner.rs'
+++

Quitting the TUI while plans are still running is treated as an interrupt (`PlanRunInterrupt::Interrupt`, exit 130). Before the interrupt work, the run continued headless with stderr redirected and no terminal output. Neither suits an operator who closes the dashboard to look at something else.

Fix: offer a detach choice that keeps the run going and falls back to inline progress on the terminal (or to a hub the operator can reattach to).
