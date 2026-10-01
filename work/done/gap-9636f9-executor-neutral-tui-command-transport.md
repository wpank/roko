+++
id = "gap-9636f9"
kind = "gap"
title = "Executor-Neutral TUI Command Transport"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/tui"]
created = 2026-09-07
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/233-tui-runner-command-channel.md#233 — Executor-Neutral TUI Command Transport"
discovered_from = "audit:tmp/backlog/archive/233-tui-runner-command-channel.md#233 — Executor-Neutral TUI Command Transport"
anchors = ["crates/roko-cli/src/execution_control.rs::ExecutionCommandSender", "crates/roko-cli/src/graph_execution/plan_runner.rs:1090", "crates/roko-cli/src/tui/app/mod.rs:189"]
links = { depends_on = [], blocks = [], related = ["gap-c002bb"], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -q "ExecutionCommandSender::channel" crates/roko-cli/src/graph_execution/plan_runner.rs'

[closed]
at = 2026-09-28
commit = "91b4745f8"
evidence = "crates/roko-cli/src/execution_control.rs defines ExecutionCommandSender + CommandAckReceiver; the TUI holds exec_cmd_sender (tui/app/mod.rs:189, 1260) and the Graph runner creates ExecutionCommandSender::channel(graph-engine) with an ack receiver (graph_execution/plan_runner.rs:1087-1090, control_adapter.rs:343); committed at HEAD. Recovery keybinding wiring remains in gap-c002bb"
+++
Executor-Neutral TUI Command Transport

Imported without verification from:
- `tmp/backlog/archive/233-tui-runner-command-channel.md#233 — Executor-Neutral TUI Command Transport`
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#Phase A: Foundation (Wave 0-1, ~3-5 days #233`

How to verify: Check: Create the exact transport types, capacities, files, and adapters above.; Replace the existing TUI sender with `ExecutionCommandSender` plus the acknowledgement receiver.; Reject full/stale/disconnected sends visibly and deterministically… [evidence: own status: Implemented (2026-09-04); 00-STATUS-SUMMARY 3. Open / P1 -- High (Open): M | engine DAG | Done (2026-09-04); (newer evidence overrides own status "closed")]

Verified 2026-09-28: implemented - graph_execution/plan_runner.rs:1090 and tui/app/mod.rs:189 use ExecutionCommandSender.
