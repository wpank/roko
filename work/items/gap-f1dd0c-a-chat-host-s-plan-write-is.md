+++
id = "gap-f1dd0c"
kind = "gap"
title = "A chat host's plan write is distilled on its planner's model, with no setting to change it: round v7's 17 plan-writing distillations sent ~12.5k tokens each for a median of 14 back, cost $0.233 ($0.032-0.036 a write on gpt-5.4), and yielded one empty knowledge entry"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan_generate"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 run-analyst (tmp/hermes-roko-v7/findings/run-analyst.md, section 7), the lead's ask"
discovered_from = "agent:run-analyst (round v7)"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan", "crates/roko-cli/src/learning_helpers.rs::distillation_model_caller_on", "crates/roko-neuro/src/episode_completion.rs::spawn_recorded_episode_distillation"]
links = { depends_on = [], blocks = [], related = ["gap-143b5c", "bug-5b04d3", "bug-a0432b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_writing_distillation_uses_the_cheapest_tool_loop_model' crates/roko-cli/src && cargo test -p roko-cli --lib plan_writing_distillation_uses_the_cheapest_tool_loop_model"
+++

## Problem

After each plan write, roko distils the write's episode in the background (role `episode-distiller`).
- In round v7 that was 17 calls and $0.233. Each sent 12.4–12.9k input tokens and got back 7–223 output tokens,
  a median of 14.
- They produced one knowledge entry in all, and an empty one: "A planning task with id pattern
  "plan:generate:<plan_id>" can complete successfully in a single agent turn with no recorded external actions"
  (Undertow's `neuro/knowledge.jsonl`, from phase 4.1's write).

A chat host's write is distilled by the planner's model, by code. `generate_plan` sets
`distiller = for_chat_host.then_some(planner_model)` (bug-a0432b).
- `chat_host_planner` takes the strongest tool-loop rung unless `[authoring] planner_model` is set.
- When phase 4's ladder moved to OpenAI, each distillation therefore ran on gpt-5.4: $0.032–0.036, against $0.009
  on gpt-5.4-mini.

No config key sets the distiller's model.
- Other writers use `[agent] default_model` (`distillation_model_caller_on`).
- For a chat host's write, setting `[authoring] planner_model` moves the planner and the distiller together.

Expected: plan-writing distillation is skipped when it yields nothing, or runs on the cheapest model roko's own tool
loop runs. Either way it keeps bug-a0432b's rule that a chat host's request never runs on the person's own CLI agent,
and its cost is counted in the write.

## Why it matters

Golden path, cost.
- It was $0.23 of the round's $2.01 of plan writing (12%), for one empty entry.
- On a gpt-5.4 planner it adds about 8% to every write.
- Each call also carries the whole local tool catalog (bug-5b04d3), and the write's operation leaves its cost out
  (gap-143b5c), so nobody saw it until the cost log was read row by row.

## Where

- `plan_generate/pipeline.rs::generate_plan`: `let distiller = for_chat_host.then_some(planner_model.as_str())`,
  about line 1393. It is passed to `agent_exec::persist_capture_episode_distilled_by`.
- `learning_helpers.rs::distillation_model_caller_on`: a `ModelCallService` on the given model, else
  `[agent] default_model`.
- `roko-neuro/src/episode_completion.rs::spawn_recorded_episode_distillation`: records the calls in
  `learn/costs.jsonl` under role `episode-distiller`.

## Current state

- bug-a0432b chose the planner model so that a chat host's distillation never runs on the person's CLI agent.
- Data: the 17 `episode-distiller` rows in Undertow's `learn/costs.jsonl`, whose `task_id` is
  `plan:generate:<plan>`, and `neuro/knowledge.jsonl`.
- `tmp/hermes-roko-v7/run-analyst/data/plan_writes_detail.csv` has them per write.

## Plan

1. **Skip distillation for plan-writing episodes** (task id `plan:generate:*`). Their yield here was one empty entry
   in 17. Check another workspace's yield first if one exists.
2. **If it stays**, run it on the cheapest model in `config.tool_loop_models()`, by the price snapshot, instead of the
   planner model. That still never uses the person's CLI agent. Optionally add `[authoring] distiller_model` to
   override it.
3. Count its cost in the write's operation (gap-143b5c), and drop the tool catalog from its request (bug-5b04d3).

## Done when

- A chat host's plan write makes no distillation call on the planner model. It makes none at all, or one on the
  cheapest tool-loop model. Test: `plan_writing_distillation_uses_the_cheapest_tool_loop_model`, or rename the
  test if the choice is to skip.
- The write's reported cost includes any distillation call.
- The `[[verify]]` passes.

## Notes

- Undertow is setting `[authoring] planner_model` to gpt-5.4-mini before phase 5's plan, the lead's decision. That
  also moves this distiller to gpt-5.4-mini for chat-host writes.
- Evidence: `tmp/hermes-roko-v7/findings/run-analyst.md`, section 7.
