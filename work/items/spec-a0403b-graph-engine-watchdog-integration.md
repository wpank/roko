+++
id = "spec-a0403b"
kind = "spec"
title = "Graph Engine Watchdog Integration"
status = "open"
triage = "verified"
severity = "p0"
subsystem = ["roko-cli"]
created = 2026-09-21
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/401-graph-engine-watchdog-integration.md#401 — Graph Engine Watchdog Integration"
discovered_from = "audit:tmp/backlog/archive/401-graph-engine-watchdog-integration.md#401 — Graph Engine Watchdog Integration"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs", "crates/roko-cli/src/graph_execution/plan_runner.rs", "crates/roko-cli/src/runner/conductor_adapter.rs", "crates/roko-cli/src/graph_execution/feedback.rs::ConductorSink"]
links = { depends_on = [], blocks = [], related = ["gap-ebd656", "gap-fab31c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -rqn "ConductorAdapter\|conductor_adapter" crates/roko-cli/src/graph_execution crates/roko-cli/src/graph_task_dispatch.rs'
+++
the Graph engine is the sole production executor since PR #260 and has zero behavioral safety net beyond raw `timeout_secs`. PR #260 made the Graph engine the default and sole production executor for plan execution (`roko plan run`). PR #276 retired `WorkflowEngine`. The Runner-v2 event loop is…

Imported without verification from:
- `tmp/backlog/archive/401-graph-engine-watchdog-integration.md#401 — Graph Engine Watchdog Integration`
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T0-05: Implement progress-heartbeat watchdog`
- `tmp/archive/plan-audit-2026-09-23/09-safeguards-watchdog.md`

Some cited files are gone: `crates/roko-cli/src/runner/graph_conductor.rs`, `runner/graph_conductor.rs`.

How to verify: Check: `GraphTaskDispatcher::dispatch()` and `dispatch_streaming()` push mapped signals to the conductor ring for each `GraphTaskEvent`.; The supervision ticker runs alongside graph plan execution at the configured… [evidence: no status line; no index/roll-up evidence] / grep graph_task_dispatch/graph_execution for conductor/watchdog/heartbeat.

Merged 2 mined candidates: m1-124, m3-113.

Verified 2026-09-28: still true - graph_task_dispatch.rs and graph_execution/ contain no conductor ring feed, supervision ticker or progress watchdog (only conductor.express_mode at graph_task_dispatch.rs:669 and the post-settlement ConductorSink at graph_execution/feedback.rs:602); ConductorAdapter is referenced only from runner/mod.rs, runner/types.rs, commands/do_cmd.rs (runner::run stub path) and runtime_feedback/routing.rs.
