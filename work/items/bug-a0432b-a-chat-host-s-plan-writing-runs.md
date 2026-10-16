+++
id = "bug-a0432b"
kind = "bug"
title = "A chat host's plan writing runs on a CLI agent, outside the rule its runs follow, and its operation reports no cost"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-cli/plan_generate", "roko-cli/model_selection", "roko-serve/plans"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "tmp/hermes-roko-v5/findings/dogfood-desktop.md P9 (2026-10-16, roko 1630fa1bf)"
anchors = ["crates/roko-cli/src/model_selection.rs::resolve_planner_selection", "crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan", "crates/roko-cli/src/learning_helpers.rs::distillation_model_caller", "crates/roko-serve/src/routes/plans/authoring.rs::start_plan_generation"]
links = { depends_on = [], blocks = [], related = ["bug-7a08a8", "gap-1f08f4", "gap-cf427e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_chat_host_plan_is_written_by_a_tool_loop_model' crates/roko-cli/src && grep -rqw 'fn a_chat_host_plan_is_never_written_on_a_cli_agent' crates/roko-cli/src && grep -rqw 'fn plan_generation_status_reports_its_cost' crates/roko-serve/src && cargo test -p roko-cli -p roko-serve --lib -- a_chat_host_plan_is_written_by_a_tool_loop_model a_chat_host_plan_is_never_written_on_a_cli_agent plan_generation_status_reports_its_cost"
+++

## Problem

A plan a chat host asks for is written by whatever model `[authoring] planner_model`, the strategist role or
`[agent] default_model` names, even a CLI agent. Chat-started runs may use only models roko's own tool loop runs,
and the plan writing doesn't follow that rule. And the operation the host follows reports no cost.

Seen in the 2026-10-16 desktop dogfood (tmp/hermes-roko-v5/findings/dogfood-desktop.md, P9; roko 1630fa1bf, Will's
global config): the plan writing of op `8d7bcaa1` ran `claude-sonnet-4-6` on `claude_cli` (46 s, 7 tool calls),
on the person's Claude Code login. In the same workspace the ladder skipped `claude_cli` for the chat host's task
runs ("skipping a rung whose agent runs its own tools …", gap-cf427e). The distiller that followed ran on the same
CLI and failed. The page showed the plan writing's spend as "—".

The cost is on disk: `efficiency.jsonl` has the row (`task_id = generate`, $0.139). The operation's status
(`run_status` over `/mcp`, `GET /api/operations/{id}`) carries none, so a host cannot show it or count it against
its limits (gap-1f08f4's first point).

## Why it matters

A chat host's request spends the person's subscription, not roko's API budget, and does not follow the rule its
runs follow. Its spend is invisible to the host (goal `hermes`).

## Where

- `crates/roko-cli/src/model_selection.rs::resolve_planner_selection`: the planner's precedence chain.
- `crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan`: the planner, its retries' escalation, and the
  episode capture that triggers distillation.
- `crates/roko-cli/src/learning_helpers.rs::distillation_model_caller`: distils on `[agent] default_model`.
- `crates/roko-cli/src/plan_authoring.rs::AuthoringSpend`: records each call's spend.
- `crates/roko-serve/src/routes/plans/authoring.rs::start_plan_generation`: the operation's result.

## Plan

1. A chat host's plan is written by a model roko's own tool loop runs: the planner the chain picks when it
   qualifies, else the highest qualifying ladder rung, else the first qualifying model
   (`RokoConfig::tool_loop_models`); with none, generation fails with the reason, as a chat run is refused
   (`no_tool_loop_model`). Retries escalate only to such models, and the distiller runs on the planner model.
2. Generation sums its calls' spend. serve's operation result carries `cost_usd`, and a plan that is written but
   cannot run fails with its errors and what writing it cost.

## Done when

- A chat host's plan in a workspace whose planner is `claude_cli` is written by a qualifying API model, or refused
  with the reason when there is none.
- A finished plan generation's status reports its cost.
- `[[verify]]` passes.
