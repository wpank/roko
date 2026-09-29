+++
id = "bug-4641e3"
kind = "bug"
title = "Forced exit and SIGHUP end a plan run without finalizing its checkpoint"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-cli/graph_execution", "roko-cli/tui"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e2-planset"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::force_exit", "crates/roko-cli/src/tui/app/mod.rs::install_terminal_signal_cleanup"]
links = { depends_on = [], blocks = [], related = ["bug-5c2d01"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "sed -n '/^fn force_exit/,/^}/p' crates/roko-cli/src/graph_execution/plan_runner.rs | grep -q 'interrupted' && grep -q 'SignalKind::hangup' crates/roko-cli/src/graph_execution/plan_runner.rs"
+++

bug-5c2d01 fixed the graceful path: the first SIGINT/SIGTERM finalizes the in-flight plan's checkpoint as `interrupted`. The fallbacks do not. `force_exit` (after a second signal, or when the 10 s graceful deadline elapses) restores the terminal and calls `process::exit`, logging that "the interrupted plan's checkpoint may still read `running`". SIGHUP is still claimed by the TUI's reset-and-reraise handler (`install_terminal_signal_cleanup`), so hanging up the terminal kills the run without finalizing it either. Resume still works, but anything that reads the checkpoint sees a run that looks alive.

Fix: write the `interrupted` status (best effort, bounded) before a forced exit, and route SIGHUP through the plan-run interrupt handler.
