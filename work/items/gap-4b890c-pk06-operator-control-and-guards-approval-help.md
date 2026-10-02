+++
id = "gap-4b890c"
kind = "gap"
title = "PK06 Operator control and guards: --approval help and validate_graph_execution_options describe an approval mode that… (+4 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
rank = 6
size = "L"
subsystem = ["roko-cli/plan"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK06"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs", "crates/roko-cli/src/commands/plan.rs", "crates/roko-cli/src/inject/transport.rs", "crates/roko-cli/src/main.rs", "crates/roko-cli/src/state_hub_ipc.rs", "docs/v3/30-CONDUCTOR.md"]
lane = "rust-cold"
parent = "spec-65c828"
links = { depends_on = ["gap-198c9c", "gap-843aef"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'supports approval mode via GraphExecutionControlAdapter' crates/roko-cli/src/commands/plan.rs && ! grep -q 'Run with interactive TUI approval' crates/roko-cli/src/commands/plan.rs && ! grep -q 'while Runner-v2 runs' crates/roko-cli/src/commands/plan.rs"

[[verify]]
command = "grep -rqw 'fn plan_run_leaves_operator_checkout_clean' crates/roko-cli/tests/ && cargo test -p roko-cli --test operator_checkout_clean plan_run_leaves_operator_checkout_clean"

[[verify]]
command = "grep -rqw 'fn settings_json_fits_one_linux_argument' crates/roko-agent/ && cargo test -p roko-agent settings_json_fits_one_linux_argument"

[[verify]]
command = "grep -rqw 'fn hub_socket_binds_under_a_long_workspace_path' crates/roko-cli/ && cargo test -p roko-cli hub_socket_binds_under_a_long_workspace_path"

[[verify]]
command = "! grep -q 'None of them runs during plan execution' docs/v3/30-CONDUCTOR.md && grep -q 'supervise' docs/v3/30-CONDUCTOR.md"
+++

## Problem

This package delivers 5 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK06, slice 12xx, phase 1), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 1221 | S | p3 | --approval help and validate_graph_execution_options describe an approval mode that does not exist | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1221-approval-help-and-graph-approval-comment-say-what-exists.md` |
| 2 | 1222 | S | p3 | Every plan run rewrites plans/INDEX.md and the .roko indexes in the operator checkout | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1222-plan-run-leaves-the-operator-checkout-clean.md` |
| 3 | 1223 | M | p2 | The Claude CLI --settings argument is 155 KB, over Linux's 128 KiB per-argument limit | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1223-claude-cli-settings-payload-fits-one-linux-argument.md` |
| 4 | 1224 | M | p3 | The StateHub and inject sockets fail to bind when the workspace path is long | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1224-unix-sockets-bind-under-long-workspace-paths.md` |
| 5 | 1225 | S | p2 | docs/v3/30-CONDUCTOR.md says the conductor never runs, but every plan run ticks it | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1225-conductor-doc-says-what-runs.md` |

## Why it matters

Phase 1: safe runs. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1200-operator-control-held-items-off-secrets-and-guards.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-agent/src/claude_cli_agent.rs`, `crates/roko-cli/src/commands/plan.rs`, `crates/roko-cli/src/inject/transport.rs`, `crates/roko-cli/src/main.rs`, `crates/roko-cli/src/state_hub_ipc.rs`, `crates/roko-cli/tests/operator_checkout_clean.rs`, `docs/v3/30-CONDUCTOR.md`.

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

- Waits on: PK04 (gap-198c9c), PK05 (gap-843aef).
- Suggested model: opus.
