+++
id = "gap-dd89ff"
kind = "gap"
title = "The self-model has no feature for the lessons and playbooks an attempt's prompt carries, so once active it would start lesson-covered tasks on a pricier rung"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-16
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "bac5cc36e"
source = "round-v6-learning-research#4a.2"
discovered_from = "agent:learning-research (round v6, 2026-10-16)"
anchors = ["crates/roko-learn/src/self_model/features.rs::TaskFeatures", "crates/roko-cli/src/graph_task_dispatch/self_model.rs::task_features"]
lane = "rust-hot"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = ["gap-b38ed2", "gap-e464e6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn task_features_carry_the_knowledge_given' crates/roko-cli/ && cargo test -p roko-cli task_features_carry_the_knowledge_given"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T17:22:13Z"
commit = "bac5cc36e"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-17T11:04:43Z"
forced = false
evidence = "bac5cc36e takes the item's plan. roko-learn self_model/features.rs: TaskFeatures gains a content group, ContentFeatures (serde default): the chain's arms on L-know's and L-play's layers and the stored knowledge entries and playbooks the prompt carries; feature_vector sets knowledge_arm:<arm> and playbook_arm:<arm> (none without an arm set) and knowledge_carried / playbooks_carried, each capped at CONTENT_CAP = 3 (CONTENT_GROUP lists them). FEATURE_SCHEMA is now m3-features/2 and FEATURE_SCHEMA_VERSION = 2, which prediction rows name as features_schema (roko-cli's own FEATURES_SCHEMA = 1 copy is gone), so the predictor version changes and a version-1 state or calibration window is set aside and restarts (gap-b38ed2's rule). roko-cli dispatch/prompt_builder.rs: PromptAssembler::carried_content(task, ctx) runs the assembler's own knowledge and playbook readers, cached or uncached, and counts the knowledge and playbook items whose exposure rows the prompt would write (two shared topic terms, entry confidence >= 0.3, at most three each), less a section the chain's arm set or the attempt's theta withholds (arm_withholds and theta_withholds, which PromptContext now delegates to); its fault-flag reads run in the attempt's decision scope (attempt_decision), as planning's do; the sources' collect takes the workdir instead of a PromptContext. It does not see the token budget or a section-bandit drop of domain_context, which happen after composition (gap-ad9e03). graph_task_dispatch/self_model.rs: forecast_attempt counts what the prompt will carry for prompt_task(spec, task), and task_features fills the group from that and from RoutingInputs' arm set; both are known before dispatch, so the forecast never reads the outcome. The literal verify, `grep -rqw 'fn task_features_carry_the_knowledge_given' crates/roko-cli/ && cargo test -p roko-cli task_features_carry_the_knowledge_given`, passes on the gap-dd89ff branch at bac5cc36e (76 test binaries, the named test ok): two shadow-mode GraphTaskDispatcher::dispatch runs of one task, in workspaces holding one stored lesson on its topic, with L-know forced to learned in one and default in the other (L-play learned in both); the first forecast's features carry Some(Learned) and 1 entry, the second's Some(Default) and 0, all other fields are equal, the two feature vectors differ only in CONTENT_GROUP features, and the shadow run's predictions.jsonl row has features_schema 2. Its static part fails at 9c9bc7ea7 (the test did not exist). Also passing: roko-learn's 61 self_model lib tests (the_vector_carries_every_group covers the encoding and cap, state_round_trips_and_version_tracks_schema the next schema), 87 roko-cli lib tests matching self_model, prompt_builder or decision_log (withhold_arm_omits_sections_and_logs_propensity and self_model_shadow_writes_predictions_and_keeps_routing among them), default_plan_run_forecasts_in_shadow, default_plan_run_registers_m1_in_shadow and plan_run_writes_calibration_json (run_graph_plan, the path `roko plan run` takes, with the cached readers), and the 5 learn_self_model bin tests. `cargo clippy -p roko-learn -p roko-cli --no-deps -- -D warnings` is clean and `cargo +nightly fmt -p roko-learn -p roko-cli` was applied, all through the limiter. Filed gap-6d583a: roko learn self-model fit still builds offline units with TaskFeatures::from(unit), so they carry no content group."
+++

## Problem

The M3 self-model forecasts each routed attempt from task, arm and history features. None of them says whether the
attempt's prompt got a stored lesson or playbook (`TaskFeatures`: a hashed bag of words over title, description and
verify commands, family, tier, role, attempt ordinal, earlier failure and error class, the arm one-hots, a price tier,
and the (family, model) pass EMA). A lesson can turn a rung that failed into one that passes first time, and the
forecast cannot tell those cases apart.

Seen in round v6 (`tmp/hermes-roko-v6/findings/learning-research.md` §3.3–3.4; evidence under
`tmp/hermes-roko-v6/learning-research/evidence/`), shadow mode, three similar one-task plans:

- After run 1 (cheap rung failed twice, mid passed), P(verified | gpt-oss-120b) fell from 0.475 to 0.123. From run 2
  on, the self-model "would start on" glm-4.7 (`runs/*/predictions.jsonl`, `decision.would_choose`).
- In the live run (`evidence/live`) the third task's prompt carried two stored lessons, and the cheap rung passed on its
  first attempt ($0.016 against $0.068 for the first task). Its forecast for that attempt was cheap 0.050, mid 0.781:
  it would still have started on mid.

So once M3 can act (`[self_model] mode = "active"`, after gap-b38ed2), it would start lesson-covered tasks on a pricier
rung and undo the saving the lesson made.

## Why it matters

Goal `cybernetic`; round v6's pitch is "escalate only when the checks fail, and get cheaper with use". Two of roko's
loops (L-know, L-M3) would pull the start rung in opposite directions. Related: gap-b38ed2 (M3 can't act yet),
gap-e464e6 (forecast instability).

## Where

- `crates/roko-learn/src/self_model/features.rs::TaskFeatures`: the feature groups (S04 §4.2) and `features_hash`.
- `crates/roko-cli/src/graph_task_dispatch/self_model.rs::task_features`: builds them before routing, in
  `GraphTaskDispatcher::forecast_attempt`.
- The chain's arm set is drawn at attempt open, before the forecast, so the L-know and L-play arms are known then.
  Prompt assembly's retrieval (`dispatch/prompt_builder.rs::collect_neuro_knowledge_cached`, `collect_playbooks_cached`)
  is cached per task.

## Current state

No knowledge or playbook signal reaches the self-model. The features schema is versioned (`FEATURES_SCHEMA = 1`); a new
group means a new schema version, and old predictor state restarts its window (gap-b38ed2's rule).

## Plan

1. Add a "content" group to `TaskFeatures`: the L-know and L-play arms of the chain (learned or default) and the number
   of stored entries and playbooks that clear the prompt's topic floor (≥2 shared terms, confidence ≥0.3), capped at 3.
2. Fill it in `task_features` from the attempt's arm set and the cached retrieval, so it reflects what the prompt will
   carry. Bump the features schema.
3. Test `task_features_carry_the_knowledge_given`: two attempts of the same task, one with a matching stored lesson on
   the learned arm and one with it withheld, give feature vectors that differ only in the new group.

## Done when

The `[[verify]]` command passes, and a shadow run's `predictions.jsonl` rows carry the new schema version.

## Notes

The self-model must not peek at the attempt's outcome. The arm and the retrieval count are known before dispatch, and
they are exactly what changes between a lesson-covered attempt and a bare one.
