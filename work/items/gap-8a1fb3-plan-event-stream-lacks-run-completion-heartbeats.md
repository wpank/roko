+++
id = "gap-8a1fb3"
kind = "gap"
title = "Plan event stream lacks run completion, heartbeats, timestamps, DAG edges and gate rung"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["roko-cli/graph-execution", "roko-serve/events"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/portal-audit/01-FINDINGS.md#B4"
discovered_from = "doc:tmp/portal-audit/01-FINDINGS.md"
anchors = ["crates/roko-serve/src/lib.rs:1613", "crates/roko-cli/src/runner/tui_bridge.rs::TuiBridge"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

On the Graph path no run-completion event is emitted (`run_duration_ms` stays null) and agent heartbeats have no caller (`AgentState.elapsed_ms` stays 0).
`TaskState` / `PlanDisplayState` carry no timestamps, and no event carries DAG edges: only the REST prefetch `GET /api/plans/{id}/tasks` has `depends_on`.
The dashboard gate-result event drops the rung index that `ServerEvent::GateResult` carries (`crates/roko-serve/src/lib.rs:1613`), and neither has a duration.
Fix: emit a run-completion event with elapsed time (planned: `plans/portal-programme/03-backend-live-events` T05). Timestamps, heartbeats, DAG edges and gate rung/duration are in no portal plan task and need their own change.

**Partial progress (2026-09-28, plan 03):** `run_completed` is now emitted on every exit path with `outcome` and `duration_ms` (T05 passed). `agent_heartbeat` is now emitted every 5 s during a turn (T04 passed). **Still open:** timestamps on `task_started`/`task_completed`, DAG edges in any event (fetch `GET /api/plans/{id}/tasks` for `depends_on`), rung index and duration on `gate_result`. The engine's unused `GraphExecutionEvent` taxonomy is tracked separately by gap-8921a3 and reg-cbfff6.
