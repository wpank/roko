+++
id = "gap-d40bc0"
kind = "gap"
title = "Plan task status reaches the dashboard in batches, not as live transitions"
status = "done"
triage = "verified"
severity = "p2"
subsystem = ["roko-cli/graph-execution"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/portal-audit/01-FINDINGS.md#B1"
discovered_from = "doc:tmp/portal-audit/01-FINDINGS.md"
anchors = ["crates/roko-cli/src/runner/graph_tui_bridge.rs::GraphTuiBridge::poll_status_changes"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = 'grep -rn "poll_status_changes(" crates/roko-cli/src --include="*.rs" | grep -v "runner/graph_tui_bridge.rs" | grep -q .'

[closed]
at = 2026-09-28
by = "plan:portal-programme/03-backend-live-events#T02"
run_id = "graph-03-backend-live-events-0ea932f0-fc26-479a-92e0-90c2cb2648a3"
evidence = "PASS status is live: T01 completes before T02 starts — LIVE-EVENTS-CHECK: PASS (12 checks)"
+++

During a Graph plan run `task_started` is emitted for every node before execution and `task_completed` for every node in one batch after the plan finishes, so progress jumps from 0 to N.
`GraphTuiBridge::poll_status_changes` (`runner/graph_tui_bridge.rs:175`), which derives per-node transitions, is only called from its own unit test.
Fix: observe node status while the graph executes and emit each transition (with a timestamp) as it happens.
Planned in `plans/portal-programme/03-backend-live-events` T01, tested by T08.
