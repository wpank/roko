+++
id = "bug-45c355"
kind = "bug"
title = "Fix Agent Spawned Event Timing (Post-Dispatch)"
status = "open"
triage = "verified"
severity = "p1"
goal = "visibility"
subsystem = ["roko-cli"]
created = 2026-09-21
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/378-fix-agent-spawned-event-timing.md#378 — Fix Agent Spawned Event Timing (Post-Dispatch)"
discovered_from = "audit:tmp/backlog/archive/378-fix-agent-spawned-event-timing.md#378 — Fix Agent Spawned Event Timing (Post-Dispatch)"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::forward_dispatch_events_to_tui", "crates/roko-cli/src/graph_task_dispatch.rs:3750"]
links = { depends_on = [], blocks = [], related = ["bug-70fd41"], supersedes = [], duplicate_of = "" }
+++
TUI shows agent as spawned only after it finishes. The running-audit claimed this was fixed (event emitted pre-dispatch), but verification shows it's still emitted AFTER dispatch completes. `forward_dispatch_events_to_tui()` is called at line 1951 after `run_shared_agent_bridge()` returns at line…

Imported without verification from:
- `tmp/backlog/archive/378-fix-agent-spawned-event-timing.md#378 — Fix Agent Spawned Event Timing (Post-Dispatch)`

How to verify: Check whether the gap described in tmp/backlog/archive/378-fix-agent-spawned-event-timing.md still exists at the anchored paths. [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Audit Sweep Items (#376-#395)]

Verified 2026-09-28: still true - tui.agent_spawned is only called inside forward_dispatch_events_to_tui (graph_task_dispatch.rs:2613-2633), which runs after run_shared_agent_bridge returns (graph_task_dispatch.rs:3743 -> 3750, and 3366).
