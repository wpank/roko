+++
id = "gap-8921a3"
kind = "gap"
title = "Dead code paths (GraphExecutionEvent sink, duplicate streaming verify path, build_fix_prompt)"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
subsystem = ["roko-graph/engine"]
created = 2026-09-25
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code"
anchors = ["engine.rs:323", "graph_task_dispatch.rs:3463-3602", "TaskDef::build_fix_prompt", "crates/roko-graph/src/engine.rs:327", "crates/roko-cli/src/graph_task_dispatch.rs::settle_task_verification", "crates/roko-cli/src/task_parser.rs:612"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The engine stores a GraphExecutionEvent sink but never emits, so --log-file writes nothing; a second streaming verify implementation (graph_task_dispatch.rs:3463-3602) drifts with no cap/feedback/auto-fix; TaskDef::build_fix_prompt is test-only.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code`
- `tmp/archive/dogfood-audit-2026-09-03/01-findings-register.md#Implemented findings with additional context from the audits`
- `tmp/archive/dogfood-2026-08-25/DOGFOOD-DEBRIEF.md#Issue 2: Log output buffering prevents monitoring (MEDIUM)`

How to verify: Run plan run --log-file and check file content.

Partly fixed (checked 2026-09-28 against 3d0ee4d02): Fixed in 725f21e05: --log-file now writes JSONL through a StateHub-subscribing recorder (crates/roko-cli/src/graph_execution/event_log.rs, run_recorded), and the streaming path no longer has its own verify logic: both the batch path (graph_task_dispatch.rs:3545-3551) and the streaming path (:3943-3965) call the shared settle_task_verification (:1708). Still dead: the engine stores event_sink (crates/roko-graph/src/engine.rs:327) and with_event_sink sets it (:448-450; attached from graph_execution/plan_runner.rs:1886), but the engine never reads it, so no GraphExecutionEvent is emitted. TaskDef::build_fix_prompt (crates/roko-cli/src/task_parser.rs:612) is still called only from tests (:2494, :2535, inside mod tests at :1759).
