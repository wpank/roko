+++
id = "gap-038eaa"
kind = "gap"
title = "Graph plan runs never emit token usage or cost events to the dashboard"
status = "done"
triage = "verified"
severity = "p2"
subsystem = ["roko-cli/graph-dispatch", "roko-serve/events"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/portal-audit/01-FINDINGS.md#B2"
discovered_from = "doc:tmp/portal-audit/01-FINDINGS.md"
anchors = ["crates/roko-cli/src/runner/tui_bridge.rs::TuiBridge::token_usage", "crates/roko-cli/src/graph_task_dispatch.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = 'grep -q "token_usage" crates/roko-cli/src/graph_task_dispatch.rs'

[closed]
at = 2026-09-28
by = "plan:portal-programme/03-backend-live-events#T03"
run_id = "graph-03-backend-live-events-0ea932f0-fc26-479a-92e0-90c2cb2648a3"
evidence = "PASS input tokens are attributed to the task; PASS cost is attributed to the task — LIVE-EVENTS-CHECK: PASS (12 checks)"
+++

`TuiBridge::token_usage` / `efficiency_event` are reached only through the legacy runner output sink (`runner/output_sink.rs`); nothing on the Graph path (`graph_task_dispatch.rs`, `graph_execution/plan_runner.rs`) calls them.
`AgentState` token fields and `SnapshotStats.cost_usd_total` therefore stay 0 and the portal shows `$0.00` / `0 tok` for agents that ran.
The data exists: Graph dispatch already writes a `CostRecord` (plan, task, model, tokens, cost_usd, duration) to `.roko/learn/costs.jsonl`.
Fix: emit token usage and per-task cost from that cost site. Planned in `plans/portal-programme/03-backend-live-events` T02/T03, tested by T09.
