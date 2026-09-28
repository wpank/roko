+++
id = "gap-5d3b82"
kind = "gap"
title = "Proof Case 1: Agent early exit / LostEffect"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/runner"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#5.1 Proof Case 1: Agent early exit / LostEffect"
discovered_from = "audit:tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#5.1 Proof Case 1: Agent early exit / LostEffect"
anchors = ["crates/roko-runtime/src/run_ledger.rs:523", "crates/roko-cli/src/graph_task_dispatch.rs"]
links = { depends_on = [], blocks = [], related = ["gap-01b2ff"], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -rqn "LostEffect" crates/roko-cli/src/graph_execution crates/roko-cli/src/graph_task_dispatch.rs'
+++
Agent exits before its first message/tool event; runner detects `LostEffect`, releases capacity, persists a terminal, and either retries or fails within the configured deadline.

Imported without verification from:
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#5.1 Proof Case 1: Agent early exit / LostEffect`

How to verify: Source: Dogfood audit, proof case 1. Check the described code path for: Agent exits before its first message/tool event; runner detects `LostEffect`, releases capacity, persists a terminal, and either retries or fails within the…

Verified 2026-09-28: still open - LostEffect exists only in roko-runtime/src/run_ledger.rs:523/839 and the legacy runner/types.rs; the Graph path (graph_execution/, graph_task_dispatch.rs, commands/plan.rs) never uses the run ledger, so an agent that exits before its first event gets no LostEffect terminal.
