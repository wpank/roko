+++
id = "gap-b5caf3"
kind = "gap"
title = "PK32 Loops re-closed: Retire the legacy holdout from Graph runs: its learning gate gates nothing (+11 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 32
size = "L"
subsystem = ["roko-cli/learning"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK32"
anchors = ["crates/roko-cli/src/config.rs", "crates/roko-cli/src/dispatch/model_routing.rs", "crates/roko-cli/src/graph_task_dispatch/feedback.rs", "crates/roko-cli/src/graph_task_dispatch/helper_calls.rs", "crates/roko-cli/src/graph_task_dispatch/prompt_experiment.rs", "crates/roko-cli/src/graph_task_dispatch/routing_context.rs", "crates/roko-cli/src/graph_task_dispatch/verification.rs", "crates/roko-cli/src/graph_task_dispatch/wiring.rs", "crates/roko-cli/src/runtime_feedback/hindsight.rs", "crates/roko-cli/src/runtime_feedback/routing.rs", "crates/roko-cli/src/runtime_feedback/verified_knowledge.rs", "crates/roko-cli/tests/learning_wiring_census.rs", "crates/roko-cli/tests/test_runtime.rs", "crates/roko-core/src/config/learning.rs", "crates/roko-core/src/config/loader.rs", "crates/roko-core/src/config/presets.rs", "crates/roko-core/src/config/routing.rs", "crates/roko-core/src/config/validation.rs", "crates/roko-execution/src/plan_generator.rs", "crates/roko-learn/src/cascade_router.rs", "crates/roko-learn/src/episode_logger.rs", "crates/roko-learn/src/event_subscriber.rs", "crates/roko-learn/src/lib.rs", "crates/roko-learn/src/playbook.rs", "crates/roko-learn/src/prompt_experiment.rs", "crates/roko-learn/src/shadow.rs", "crates/roko-neuro/src/knowledge_store/crud.rs", "crates/roko-neuro/src/lib.rs", "roko.toml"]
lane = "rust-hot"
parent = "spec-446a41"
links = { depends_on = ["gap-cc5051", "gap-f61823"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rq 'HoldoutExperiment\\|holdout_experiment' crates/roko-cli/src/ && cargo test -p roko-cli --test learning_wiring_census"

[[verify]]
command = "! grep -rq 'shadow_runner\\|ShadowRunner' crates/roko-cli/src/"

[[verify]]
command = "grep -rqw 'fn credits_from_separate_stores_are_not_lost' crates/roko-learn/src/ && cargo test -p roko-learn credits_from_separate_stores_are_not_lost"

[[verify]]
command = "grep -rqw 'fn feedback_carries_the_routes_own_source' crates/roko-cli/src/ && cargo test -p roko-cli feedback_carries_the_routes_own_source"

[[verify]]
command = "! grep -rq 'assign_retrieval_strategy_arm\\|ensure_retrieval_strategy_experiment\\|record_retrieval_outcome' crates/roko-cli/src/"

[[verify]]
command = "! grep -rq 'fn run_learning_subscriber\\|fn query_similar_episodes' crates/"

[[verify]]
command = "grep -rqw 'fn agent_blamed_failure_counts_a_contradiction_without_weakening' crates/roko-cli/src/ && cargo test -p roko-cli agent_blamed_failure_counts_a_contradiction_without_weakening"

[[verify]]
command = "! grep -q 'generate_post_gate_reflection' crates/roko-cli/src/graph_task_dispatch/verification.rs && ! grep -rq 'replan_on_gate_failure' crates/roko-cli/src/graph_task_dispatch/ crates/roko-cli/src/graph_execution/ && grep -rqw 'fn verify_failure_makes_no_reflection_call' crates/roko-cli/src/ && cargo test -p roko-cli verify_failure_makes_no_reflection_call"

[[verify]]
command = "grep -q 'learning.replan_on_gate_failure' crates/roko-core/src/config/loader.rs && ! grep -rq 'pub replan_on_gate_failure' crates/roko-core/src/ crates/roko-cli/src/ && ! grep -q '^replan_on_gate_failure' roko.toml && cargo test -p roko-core dead_config_keys_are_removed_and_old_files_still_load"

[[verify]]
command = "grep -rqw 'fn ladder_outcome_recorded_apart_from_router_picks' crates/roko-cli/ && cargo test -p roko-cli ladder_outcome_recorded_apart_from_router_picks"

[[verify]]
command = "grep -rqw 'fn ineligible_pick_is_masked_before_argmax' crates/roko-cli/src/dispatch/ && cargo test -p roko-cli ineligible_pick_is_masked_before_argmax"

[[verify]]
command = "grep -rqw 'fn route_propensities_sum_to_one_and_respect_epsilon' crates/roko-learn/src/ && cargo test -p roko-learn route_propensities_sum_to_one_and_respect_epsilon"
+++

## Problem

This package delivers 12 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK32, slice 41xx, phase 4), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 4101 | S | p2 | Retire the legacy holdout from Graph runs: its learning gate gates nothing | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4101-retire-legacy-holdout-from-graph-runs.md` |
| 2 | 4102 | S | p3 | Remove the ShadowRunner that every plan run builds and never uses | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4102-remove-unused-shadowrunner-from-plan-runs.md` |
| 3 | 4103 | S | p2 | Playbook credits from parallel tasks overwrite each other | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4103-playbook-credits-from-parallel-tasks-lose-updates.md` |
| 4 | 4104 | S | p2 | Feedback must carry the route's own source, not a recomputed one | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4104-feedback-carries-the-routes-own-source.md` |
| 5 | 4105 | S | p3 | Delete the retrieval-strategy A/A experiment | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4105-delete-retrieval-strategy-aa-experiment.md` |
| 6 | 4106 | S | p3 | Retire two learning loops nothing calls: the learning event subscriber and similar-episode recall | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4106-retire-learning-subscriber-and-similar-episode-recall.md` |
| 7 | 4107 | S | p3 | Knowledge an agent-blamed failure consulted gets a contradiction count, never a penalty | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4107-failed-attempt-knowledge-gets-contradiction-count.md` |
| 8 | 4109 | S | p2 | Stop generating post-gate reflections that nothing reads | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4109-stop-generating-post-gate-reflections.md` |
| 9 | 4110 | S | p3 | Remove the `[learning] replan_on_gate_failure` config key | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4110-remove-replan-on-gate-failure-config-key.md` |
| 10 | 4112 | S | p2 | Keep ladder outcomes apart from the router's own picks | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4112-keep-ladder-outcomes-apart-from-router-picks.md` |
| 11 | 4113 | M | p2 | Mask ineligible models inside the cascade, before its argmax | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4113-mask-ineligible-models-before-cascade-argmax.md` |
| 12 | 4114 | M | p2 | Explore with ε over the eligible models and log every candidate's propensity | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4114-epsilon-exploration-with-logged-propensities.md` |

## Why it matters

Phase 4: loops re-closed (S02). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4100-loops-reclosed-on-verified-outcomes.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/config.rs`, `crates/roko-cli/src/dispatch/model_routing.rs`, `crates/roko-cli/src/graph_execution/plan_runner.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, `crates/roko-cli/src/graph_task_dispatch/feedback.rs`, `crates/roko-cli/src/graph_task_dispatch/helper_calls.rs`, `crates/roko-cli/src/graph_task_dispatch/prompt_experiment.rs`, `crates/roko-cli/src/graph_task_dispatch/routing_context.rs`, `crates/roko-cli/src/graph_task_dispatch/verification.rs`, `crates/roko-cli/src/graph_task_dispatch/wiring.rs`, `crates/roko-cli/src/runtime_feedback/hindsight.rs`, `crates/roko-cli/src/runtime_feedback/routing.rs`, `crates/roko-cli/src/runtime_feedback/verified_knowledge.rs`, `crates/roko-cli/tests/learning_wiring_census.rs`, `crates/roko-cli/tests/test_runtime.rs`, `crates/roko-core/src/config/learning.rs`, `crates/roko-core/src/config/loader.rs`, `crates/roko-core/src/config/presets.rs`, `crates/roko-core/src/config/routing.rs`, `crates/roko-core/src/config/validation.rs`, `crates/roko-execution/src/plan_generator.rs`, `crates/roko-learn/src/cascade_router.rs`, `crates/roko-learn/src/episode_logger.rs`, `crates/roko-learn/src/event_subscriber.rs`, `crates/roko-learn/src/lib.rs`, `crates/roko-learn/src/playbook.rs`, `crates/roko-learn/src/prompt_experiment.rs`, `crates/roko-learn/src/shadow.rs`, `crates/roko-neuro/src/knowledge_store/crud.rs`, `crates/roko-neuro/src/lib.rs`, `roko.toml`.

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

- Waits on: PK09 (gap-cc5051), PK10 (gap-f61823).
- Suggested model: opus.
