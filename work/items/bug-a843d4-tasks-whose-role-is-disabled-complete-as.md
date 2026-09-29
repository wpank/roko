+++
id = "bug-a843d4"
kind = "bug"
title = "Tasks whose role is disabled complete as successes without running their verify steps"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e1-verdict"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::dispatch", "crates/roko-cli/src/config_helpers.rs::is_role_enabled"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -rq "fn disabled_role_task_" crates/roko-cli/src && cargo test -p roko-cli --lib disabled_role_task_'
+++

When `[agent.roles.<role>] enabled = false`, `GraphTaskDispatcher::dispatch` logs "role is disabled in config; skipping task" and returns `Ok(Vec::new())` (graph_task_dispatch.rs:3198-3214). The engine records a completed node, the task's verify steps never run, and the plan can report success for work that was never done. Only resume notices: the verdict-less record is re-run.

Fix: fail the task, or give it a distinct skipped status that plan success does not count as passed. Add a test named `disabled_role_task_*`.
