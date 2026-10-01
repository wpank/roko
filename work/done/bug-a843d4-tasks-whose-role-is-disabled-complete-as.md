+++
id = "bug-a843d4"
kind = "bug"
title = "Tasks whose role is disabled complete as successes without running their verify steps"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "1bf49188d"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e1-verdict"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::dispatch", "crates/roko-cli/src/config_helpers.rs::is_role_enabled", "crates/roko-cli/src/graph_execution/plan_runner.rs::disabled_role_plan_set"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -rq "fn disabled_role_task_" crates/roko-cli/src && cargo test -p roko-cli --lib disabled_role_task_'

[closed]
at = 2026-10-01
at_ts = "2026-10-01T09:00:20Z"
by = "coordinator (session 7622b882)"
forced = false
evidence = "Batch 20c gate on fcdaf32ae/ca5645373 (MAIN 1bf49188d has the same crates and portal): check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-acp/agent/cli/core/dreams/gate/learn/neuro/serve; lib tests roko-cli 3273, roko-agent 2278, roko-core 1962, roko-learn 1209, roko-serve 989, roko-gate 692, roko-neuro 239, roko-acp 199, roko-dreams 100 all pass; extras: C1 1/1, C7 2/2, learn_paths 7, cost_comparison 1, bin 429, verify loop 10/10, speclint 91; a disabled role fails the task without dispatch; C1 passes. Merged 4c0e5646e."
+++

When `[agent.roles.<role>] enabled = false`, `GraphTaskDispatcher::dispatch` logs "role is disabled in config; skipping task" and returns `Ok(Vec::new())` (graph_task_dispatch.rs:3198-3214). The engine records a completed node, the task's verify steps never run, and the plan can report success for work that was never done. Only resume notices: the verdict-less record is re-run.

Fix: fail the task, or give it a distinct skipped status that plan success does not count as passed. Add a test named `disabled_role_task_*`.

2026-09-29: re-verified at d9e79e9d8. Still open; the skip branch is now graph_task_dispatch.rs:3195-3211. The plan_runner.rs tests built on disabled_role_plan_set (lines 2418-2604) rely on disabled-role tasks completing without dispatch, so the fix must give them another no-dispatch fixture.
