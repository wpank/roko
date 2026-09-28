+++
id = "bug-70fd41"
kind = "bug"
title = "TUI shows model \"-\": resolved model not forwarded to tui.agent_spawned()"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/tui"]
created = 2026-08-13
updated = 2026-09-28
source = "tmp/archive/08-15-26/MASTER-TASKS.md#2. Runtime Bugs (M2) + 3. Runner v2 Completion"
discovered_from = "audit:tmp/archive/08-15-26/MASTER-TASKS.md#2. Runtime Bugs (M2) + 3. Runner v2 Completion"
anchors = ["agent_spawned"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Runner v2 passes `model: String::new()` to `tui.agent_spawned()` although the model is resolved, so the TUI shows "-" for model. Related later item: backlog #378 (agent spawned event timing).

Imported without verification from:
- `tmp/archive/08-15-26/MASTER-TASKS.md#2. Runtime Bugs (M2) + 3. Runner v2 Completion`

How to verify: Check agent-spawned event emission in graph_execution and runner for an empty model string; check TUI agent table model column.
