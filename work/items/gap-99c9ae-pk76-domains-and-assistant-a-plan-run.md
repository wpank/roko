+++
id = "gap-99c9ae"
kind = "gap"
title = "PK76 Domains and assistant: A plan.run cell: a trigger can run a plan (+7 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "features"
rank = 76
size = "L"
subsystem = ["roko-cli/graph"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK76"
anchors = ["crates/roko-agent/src/dispatcher/mod.rs", "crates/roko-agent/src/mcp/client.rs", "crates/roko-agent/src/mcp/to_tool_def.rs", "crates/roko-agent/src/safety/mod.rs", "crates/roko-cli/src/commands/graph.rs", "crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs", "crates/roko-cli/src/lib.rs", "crates/roko-cli/src/main.rs", "crates/roko-cli/src/orchestrator/mod.rs", "crates/roko-cli/src/serve_runtime.rs", "crates/roko-cli/src/task_parser.rs", "crates/roko-core/src/config/schema.rs", "crates/roko-core/src/task.rs", "crates/roko-fs/src/layout.rs", "crates/roko-serve/src/routes/mod.rs", "crates/roko-serve/src/routes/route_permissions.rs", "crates/roko-serve/src/runtime.rs", "examples/graphs/task-execution.toml"]
lane = "rust-cold"
parent = "spec-0b3a32"
links = { depends_on = ["gap-ce1d11", "gap-3c3729"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn graph_run_plan_cell_runs_the_plan' crates/roko-cli/ && cargo test -p roko-cli graph_run_plan_cell_runs_the_plan"

[[verify]]
command = "test -f examples/graphs/trigger-agent-task.toml && grep -rqw 'fn trigger_fires_agent_task_graph' crates/roko-cli/ && cargo test -p roko-cli trigger_fires_agent_task_graph"

[[verify]]
command = "grep -rqw 'fn destructive_open_world_mcp_tool_is_outbound_effect' crates/roko-agent/ && cargo test -p roko-agent destructive_open_world_mcp_tool_is_outbound_effect"

[[verify]]
command = "grep -rqw 'fn outbound_effect_tool_call_is_held_for_approval' crates/roko-agent/ && cargo test -p roko-agent outbound_effect_tool_call_is_held_for_approval"

[[verify]]
command = "grep -rqw 'fn approved_effect_is_applied_once_and_receipt_checked' crates/roko-cli/ && cargo test -p roko-cli approved_effect_is_applied_once_and_receipt_checked"

[[verify]]
command = "grep -rqw 'fn effect_decision_route_applies_an_approved_effect' crates/roko-serve/ && cargo test -p roko-serve effect_decision_route_applies_an_approved_effect"

[[verify]]
command = "grep -rqw 'fn task_workspace_kind_parses_scratch_dir' crates/roko-cli/ && cargo test -p roko-cli task_workspace_kind_parses_scratch_dir"

[[verify]]
command = "grep -rqw 'fn scratch_dir_attempt_gets_cow_copy_and_manifest' crates/roko-cli/ && cargo test -p roko-cli scratch_dir_attempt_gets_cow_copy_and_manifest"
+++

## Problem

This package delivers 8 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK76, slice 91xx, phase 9), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9128 | M | p3 | A plan.run cell: a trigger can run a plan | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9128-plan-run-cell-for-graph-run.md` |
| 2 | 9129 | S | p3 | An example trigger graph, and a test that a fired trigger starts an agent.task | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9129-example-trigger-graph-and-agent-task-test.md` |
| 3 | 9130 | S | p2 | Mark tools whose calls have outbound effects | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9130-mark-outbound-effect-tools.md` |
| 4 | 9131 | M | p2 | Hold outbound-effect tool calls for approval instead of running them | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9131-hold-outbound-effect-calls-for-approval.md` |
| 5 | 9132 | M | p2 | roko effects: apply an approved staged effect exactly once, then check its receipt | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9132-roko-effects-apply-once-and-check-receipt.md` |
| 6 | 9133 | M | p3 | Serve routes for staged effects: list them, and approve or reject one | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9133-serve-routes-for-staged-effects.md` |
| 7 | 9134 | S | p3 | Tasks and domain profiles declare a workspace kind: git_worktree or scratch_dir | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9134-tasks-declare-a-workspace-kind.md` |
| 8 | 9135 | M | p3 | scratch_dir attempt workspace: a copy-on-write copy of the task's data, hashed before and after | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9135-scratch-dir-attempt-workspace.md` |

## Why it matters

Phase 9: domains, assistant, held and parked work, cleanup, showcase and deploy. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9100-domains-and-the-assistant.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-agent/src/dispatcher/mod.rs`, `crates/roko-agent/src/mcp/client.rs`, `crates/roko-agent/src/mcp/to_tool_def.rs`, `crates/roko-agent/src/safety/effects.rs`, `crates/roko-agent/src/safety/mod.rs`, `crates/roko-cli/src/commands/effects.rs`, `crates/roko-cli/src/commands/graph.rs`, `crates/roko-cli/src/effects_apply.rs`, `crates/roko-cli/src/graph_entry_cells.rs`, `crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs`, `crates/roko-cli/src/lib.rs`, `crates/roko-cli/src/main.rs`, `crates/roko-cli/src/orchestrator/mod.rs`, `crates/roko-cli/src/orchestrator/scratch.rs`, `crates/roko-cli/src/serve_runtime.rs`, `crates/roko-cli/src/task_parser.rs`, `crates/roko-core/src/config/schema.rs`, `crates/roko-core/src/task.rs`, `crates/roko-fs/src/layout.rs`, `crates/roko-serve/src/routes/effects.rs`, `crates/roko-serve/src/routes/mod.rs`, `crates/roko-serve/src/routes/route_permissions.rs`, `crates/roko-serve/src/runtime.rs`, `examples/graphs/task-execution.toml`, `examples/graphs/trigger-agent-task.toml`.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: PK74 (gap-ce1d11), PK75 (gap-3c3729).
- Suggested model: opus.

## Progress

- 9128: implemented at 6265a21d5
- 9129: implemented at 1fb68cf33
- 9130: implemented at 18d97557d
- 9131: implemented at c2156c1d0
- 9132: implemented at 28b772f6c
- 9133: implemented at 297d4c8d5
- 9134: implemented at cd56ccf88
- 9135: implemented at c6c0f20c8

The Rust tasks were checked statically (verify greps, hand formatting); cargo verification is left to the batch gate. Deviations: 9128 runs plans through run_graph_plan_in_run in its own runtime (no serve-style git init); 9129 makes shell.exec pass its inputs on and gives execute_graph the workspace (serve passes its workdir); 9131 carries the outbound policy as an OutboundEffects governance rule in the task contract, set from [meta] outbound (chat runs: stage), the domain profile, or the ops default; 9135 keeps an accepted scratch directory until 9136 copies changes back, and only the batch dispatch path builds scratch workspaces.
