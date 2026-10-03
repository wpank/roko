+++
id = "gap-997366"
kind = "gap"
title = "PK14 Defaults that apply: A plan budget without max_turn_usd reserves a share per call, and any wait is logged (+8 more)"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
rank = 14
size = "L"
subsystem = ["roko-cli/graph-execution"]
created = 2026-10-02
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "059450273"
source = "tmp/backlog/2026-10-02-complete-and-wire PK14"
anchors = ["crates/roko-acp/src/bridge_events/slash_commands.rs", "crates/roko-acp/src/bridge_events/tests.rs", "crates/roko-cli/src/commands/plan.rs", "crates/roko-cli/src/dispatch/factory.rs", "crates/roko-cli/src/dispatch/mod.rs", "crates/roko-cli/src/dispatch/model_routing.rs", "crates/roko-cli/src/dispatch/prompt_builder.rs", "crates/roko-cli/src/graph_checkpoint.rs", "crates/roko-cli/src/graph_execution/delivery.rs", "crates/roko-cli/src/graph_task_dispatch/budget.rs", "crates/roko-cli/src/graph_task_dispatch/routing_context.rs", "crates/roko-cli/src/graph_task_dispatch/streaming.rs", "crates/roko-cli/src/task_parser.rs", "crates/roko-cli/tests/scheduler_canary.rs", "plans/INDEX.md", "plans/add-plan-queue/tasks.toml", "plans/e2e-smoke-test/tasks.toml", "plans/portal-plan-execution/tasks.toml", "plans/portal-programme/01-backend-plan-service/tasks.toml", "plans/portal-programme/02-backend-plan-execution/tasks.toml", "plans/portal-programme/03-backend-live-events/tasks.toml", "plans/portal-programme/03b-backend-workspace-server/tasks.toml", "plans/portal-programme/03c-backend-local-access/tasks.toml", "plans/portal-programme/04-backend-plan-authoring/tasks.toml", "plans/portal-programme/04b-backend-plan-revision/tasks.toml", "plans/portal-programme/05-portal-foundation/tasks.toml", "plans/portal-programme/06-portal-shell/tasks.toml", "plans/portal-programme/07-portal-compose/tasks.toml", "plans/portal-programme/08-portal-run/tasks.toml", "plans/portal-programme/08b-portal-polish/tasks.toml", "plans/portal-programme/08c-portal-live-steps/tasks.toml", "plans/portal-programme/08d-portal-legibility/tasks.toml", "plans/portal-programme/08e-portal-refine/tasks.toml", "plans/portal-programme/08f-final-polish/tasks.toml", "plans/portal-programme/08g-first-run/tasks.toml", "plans/portal-programme/09-acceptance/tasks.toml", "plans/qa-workflow-validation/tasks.toml", "plans/wire-http-plan-execute/tasks.toml", "plans/workspace-doctor-improvements/tasks.toml"]
lane = "rust-hot"
parent = "spec-fef7c5"
links = { depends_on = ["gap-198c9c"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_budget_without_turn_cap_runs_tasks_in_parallel' crates/roko-cli/ && cargo test -p roko-cli plan_budget_without_turn_cap_runs_tasks_in_parallel"

[[verify]]
command = "grep -rqw 'fn scheduler_canary_under_a_plan_budget' crates/roko-cli/tests/ && cargo test -p roko-cli --test scheduler_canary scheduler_canary_under_a_plan_budget"

[[verify]]
command = "! grep -q 'do not run plans in parallel yet' crates/roko-cli/src/graph_execution/plan_runner.rs"

[[verify]]
command = "grep -rqw 'fn worktree_plans_run_side_by_side_and_both_deliver' crates/roko-cli/src/ && cargo test -p roko-cli worktree_plans_run_side_by_side_and_both_deliver"

[[verify]]
command = "test -z \"$(git grep -l '^model_hint' -- 'plans/**/tasks.toml' ':!plans/archive/**')\""

[[verify]]
command = "test -z \"$(git grep -l '^max_parallel *= *1\\b' -- 'plans/**/tasks.toml' ':!plans/archive/**' ':!plans/demos/**')\""

[[verify]]
command = "! grep -A8 '\"plan-run\" =>' crates/roko-acp/src/bridge_events/slash_commands.rs | grep -q '\"--model\"' "

[[verify]]
command = "grep -rqw 'fn plan_run_slash_command_leaves_model_choice_to_the_ladder' crates/roko-acp/ && cargo test -p roko-acp plan_run_slash_command_leaves_model_choice_to_the_ladder"

[[verify]]
command = "grep -rn 'with_default_slug(' crates/roko-cli/src --include='*.rs' | grep -v 'model_routing.rs' | grep -q ."

[[verify]]
command = "grep -rqw 'fn router_default_follows_agent_default_model' crates/roko-cli/src/ && cargo test -p roko-cli router_default_follows_agent_default_model"

[[verify]]
command = "! grep -rqE 'fn effective_bias|fn arbitrate_cross_cut_routing_bias|fn dream_routing_bias|routing_bias:' crates/roko-cli/src/dispatch crates/roko-cli/src/graph_task_dispatch crates/roko-cli/src/graph_task_dispatch.rs && cargo check -p roko-cli"

[[verify]]
command = "grep -rqw 'fn workspace_context_reports_attempt_branch' crates/roko-cli/ && cargo test -p roko-cli workspace_context_reports_attempt_branch"

[[verify]]
command = "grep -rqw 'fn delivery_receipt_records_the_checks_that_ran' crates/roko-cli/ && cargo test -p roko-cli delivery_receipt_records_the_checks_that_ran"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T04:05:55Z"
commit = "059450273"
executor = "claude-agent"
via = "work-batch"
size = "L"
claimed_at = "2026-10-03T01:27:36Z"
forced = false
evidence = "Gate 6b (work/backlog-batch-6b, merged into main as 059450273): cargo check --workspace --tests, nightly fmt, cargo clippy --workspace -D warnings on the chainless default build, nextest --lib 11,842 passed over 10 crates (roko-acp, -agent, -agent-server, -cli, -core, -gate, -graph, -learn, -runtime, -serve), roko-cli bin 429 passed and the golden-path canaries pass (plan_validate: only bug-2a31bc's two known alias tests fail), roko-agent sse_replay + provider_parity and roko-learn legacy_rule_live + loop_audit_cs_reference pass, PK79's CI feature checks pass; every [[verify]] passes. PK14 9/9 (RoutingBias removed from the plan path, 24 plans edited); scheduler_canary_under_a_plan_budget and worktree_plans_run_side_by_side_and_both_deliver pass."
+++

## Problem

This package delivers 9 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK14, slice 31xx, phase 3), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3102 | M | p2 | A plan budget without max_turn_usd reserves a share per call, and any wait is logged | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3102-plan-budget-without-turn-cap-runs-in-parallel.md` |
| 2 | 3103 | S | p2 | Canary C6 under a plan budget with no turn cap | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3103-canary-c6-under-a-plan-budget.md` |
| 3 | 3104 | M | p2 | The plans of a set run side by side when per-task worktrees are on | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3104-plan-sets-run-side-by-side-under-worktrees.md` |
| 4 | 3105 | S | p2 | Free the active tracked plans from model pins and max_parallel = 1 (scripted) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3105-free-active-plans-from-model-pins-and-serial-width.md` |
| 5 | 3106 | S | p2 | ACP /plan-run leaves the model choice to the ladder instead of pinning the chat's model | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3106-acp-plan-run-leaves-model-choice-to-the-ladder.md` |
| 6 | 3107 | S | p3 | The router's fallback model follows agent.default_model | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3107-router-default-follows-agent-default-model.md` |
| 7 | 3109 | M | p3 | Remove RoutingBias from the plan path: the bias the health-aware pick drops, its arbitration and its dream-bias builder | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3109-routing-bias-applies-with-provider-health.md` |
| 8 | 3110 | S | p3 | The prompt's workspace context reports the attempt's own branch and changes | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3110-workspace-context-reports-the-attempt-branch.md` |
| 9 | 3111 | M | p2 | The whole-plan check's receipt records which checks ran and what they printed | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3111-delivery-receipt-records-the-checks-that-ran.md` |

## Why it matters

Phase 3: golden-path proof. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3100-defaults-that-apply-parallelism-ladder-integration.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-acp/src/bridge_events/slash_commands.rs`, `crates/roko-acp/src/bridge_events/tests.rs`, `crates/roko-cli/src/commands/plan.rs`, `crates/roko-cli/src/dispatch/factory.rs`, `crates/roko-cli/src/dispatch/mod.rs`, `crates/roko-cli/src/dispatch/model_routing.rs`, `crates/roko-cli/src/dispatch/prompt_builder.rs`, `crates/roko-cli/src/graph_checkpoint.rs`, `crates/roko-cli/src/graph_execution/delivery.rs`, `crates/roko-cli/src/graph_execution/plan_runner.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, `crates/roko-cli/src/graph_task_dispatch/budget.rs`, `crates/roko-cli/src/graph_task_dispatch/routing_context.rs`, `crates/roko-cli/src/graph_task_dispatch/streaming.rs`, `crates/roko-cli/src/task_parser.rs`, `crates/roko-cli/tests/scheduler_canary.rs`, `plans/INDEX.md`, `plans/add-plan-queue/tasks.toml`, `plans/e2e-smoke-test/tasks.toml`, `plans/portal-plan-execution/tasks.toml`, `plans/portal-programme/01-backend-plan-service/tasks.toml`, `plans/portal-programme/02-backend-plan-execution/tasks.toml`, `plans/portal-programme/03-backend-live-events/tasks.toml`, `plans/portal-programme/03b-backend-workspace-server/tasks.toml`, `plans/portal-programme/03c-backend-local-access/tasks.toml`, `plans/portal-programme/04-backend-plan-authoring/tasks.toml`, `plans/portal-programme/04b-backend-plan-revision/tasks.toml`, `plans/portal-programme/05-portal-foundation/tasks.toml`, `plans/portal-programme/06-portal-shell/tasks.toml`, `plans/portal-programme/07-portal-compose/tasks.toml`, `plans/portal-programme/08-portal-run/tasks.toml`, `plans/portal-programme/08b-portal-polish/tasks.toml`, `plans/portal-programme/08c-portal-live-steps/tasks.toml`, `plans/portal-programme/08d-portal-legibility/tasks.toml`, `plans/portal-programme/08e-portal-refine/tasks.toml`, `plans/portal-programme/08f-final-polish/tasks.toml`, `plans/portal-programme/08g-first-run/tasks.toml`, `plans/portal-programme/09-acceptance/tasks.toml`, `plans/qa-workflow-validation/tasks.toml`, `plans/wire-http-plan-execute/tasks.toml`, `plans/workspace-doctor-improvements/tasks.toml`.

It also edits the hot file(s) `crates/roko-cli/src/graph_execution/plan_runner.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, which are left out of this item's anchors so that two hot packages can run at once; the coordinator resolves any merge conflict.

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

- Waits on: PK04 (gap-198c9c).
- Suggested model: opus.

## Progress

Implemented on `work/gap-997366`; cargo verification deferred to the batch gate. The static part of every verify
passes.

- 3102: implemented at 90cff58c1
- 3103: implemented at 3ba95616f (the check that it fails with the whole-budget reservation put back needs cargo)
- 3104: implemented at 9c371e146
- 3105: implemented at f840012dc (`roko plan index --check` passes; the failed checkpoints of
  portal-programme/02-backend-plan-execution and 09-acceptance in the main checkout need `--fresh`)
- 3106: implemented at a989cd094
- 3107: implemented at 6d111701d
- 3109: implemented at e9d37c7c4
- 3110: implemented at 840e04b5b
- 3111: implemented at f20db587b
