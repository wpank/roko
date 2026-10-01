+++
id = "bug-a0e820"
kind = "bug"
title = "DF-0925 P1-4: Task status wrong in both directions (all started up front, completions retroactive)"
status = "superseded"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/graph_tui_bridge"]
created = 2026-09-25
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P1-4. Task status is a lie in both directions"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P1-4. Task status is a lie in both directions"
anchors = ["crates/roko-cli/src/runner/graph_tui_bridge.rs::GraphTuiBridge::poll_status_changes", "crates/roko-cli/src/graph_execution/plan_runner.rs:1431"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "gap-d40bc0" }

[closed]
at = 2026-09-28
evidence = "duplicate of gap-d40bc0 (verified 2026-09-28): plan_runner.rs still calls node_started for every task before engine.start(), GraphTuiBridge::emit_graph_output publishes completions after the plan, and poll_status_changes is called only from its unit test."
+++
node_started fires for every task before execution and completions are emitted after the whole plan; poll_status_changes, which would fix this, has zero production callers.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P1-4. Task status is a lie in both directions`

How to verify: grep poll_status_changes callers.

Verified 2026-09-28: closed as duplicate; see [closed].evidence.
