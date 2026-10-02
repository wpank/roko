+++
id = "gap-d254a3"
kind = "gap"
title = "PK03 Failure paths: A plan-branch conflict tells the next attempt what it conflicted with (+5 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
rank = 3
size = "L"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK03"
anchors = ["crates/roko-cli/src/graph_execution/workspaces.rs", "crates/roko-cli/src/graph_task_dispatch/attempt.rs", "crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs", "crates/roko-cli/src/graph_task_dispatch/failover.rs", "crates/roko-cli/src/graph_task_dispatch/red_flags.rs", "crates/roko-cli/src/graph_task_dispatch/watchdog.rs", "crates/roko-cli/src/orchestrator/worktree/mod.rs", "crates/roko-cli/src/runner/tui_bridge.rs", "crates/roko-core/src/config/schema.rs", "crates/roko-learn/src/telemetry/records.rs"]
lane = "rust-hot"
parent = "spec-65c828"
links = { depends_on = ["gap-e00238"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn conflict_retry_prompt_names_the_conflict' crates/roko-cli/ && cargo test -p roko-cli conflict_retry_prompt_names_the_conflict"

[[verify]]
command = "grep -rqw 'fn outside_scope_ignores_sibling_accepted_commits' crates/roko-cli/ && cargo test -p roko-cli outside_scope_ignores_sibling_accepted_commits"

[[verify]]
command = "grep -rqE 'scope_findings|out_of_scope' crates/roko-learn/src/telemetry/records.rs"

[[verify]]
command = "grep -rqw 'fn scope_findings_reach_the_attempt_verdict' crates/roko-cli/ && cargo test -p roko-cli scope_findings_reach_the_attempt_verdict"

[[verify]]
command = "grep -rqw 'fn hung_codex_attempt_is_cancelled_by_stall_watchdog' crates/roko-cli/ && cargo test -p roko-cli hung_codex_attempt_is_cancelled_by_stall_watchdog"

[[verify]]
command = "grep -rqw 'fn agent_spawned_names_planned_provider' crates/roko-cli/ && cargo test -p roko-cli agent_spawned_names_planned_provider"

[[verify]]
command = "grep -rqw 'fn failover_publishes_fallback_slug_to_hub' crates/roko-cli/ && cargo test -p roko-cli failover_publishes_fallback_slug_to_hub"
+++

## Problem

This package delivers 6 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK03, slice 11xx, phase 1), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 1123 | S | p2 | A plan-branch conflict tells the next attempt what it conflicted with | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1123-conflict-retry-names-the-conflict.md` |
| 2 | 1124 | S | p2 | An attempt's diff starts from the commit its worktree was created from, so siblings' accepted work is not flagged | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1124-attempt-diff-base-is-worktree-start.md` |
| 3 | 1125 | M | p3 | Out-of-scope findings are recorded on the attempt verdict, not only logged | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1125-persist-scope-findings-on-verdicts.md` |
| 4 | 1126 | M | p2 | The stall watchdog bounds Codex CLI and Cursor CLI attempts that report nothing until they finish | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1126-watch-cli-attempts-that-report-at-end.md` |
| 5 | 1127 | S | p3 | The pre-dispatch `agent_spawned` event names the planned provider, not a backend family | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1127-agent-spawned-names-planned-provider.md` |
| 6 | 1128 | S | p3 | After a failover the dashboard shows the model and provider that ran | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1128-failover-updates-dashboard-model.md` |

## Why it matters

Phase 1: safe runs. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/1100-make-provider-failures-safe.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/graph_execution/workspaces.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, `crates/roko-cli/src/graph_task_dispatch/attempt.rs`, `crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs`, `crates/roko-cli/src/graph_task_dispatch/failover.rs`, `crates/roko-cli/src/graph_task_dispatch/red_flags.rs`, `crates/roko-cli/src/graph_task_dispatch/watchdog.rs`, `crates/roko-cli/src/orchestrator/worktree/mod.rs`, `crates/roko-cli/src/runner/tui_bridge.rs`, `crates/roko-core/src/config/schema.rs`, `crates/roko-learn/src/telemetry/records.rs`.

It also edits the hot file(s) `crates/roko-cli/src/graph_task_dispatch.rs`, which are left out of this item's anchors so that two hot packages can run at once; the coordinator resolves any merge conflict.

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

- Waits on: PK02 (gap-e00238).
- Suggested model: opus.
