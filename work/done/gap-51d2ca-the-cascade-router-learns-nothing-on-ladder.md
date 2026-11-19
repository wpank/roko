+++
id = "gap-51d2ca"
kind = "gap"
title = "The cascade router learns nothing on ladder-routed plan runs: every route has propensity 1.0 and decision 4111 (A) credits only its own picks"
status = "done"
triage = "verified"
severity = "p3"
goal = "learning"
size = "M"
subsystem = ["roko-cli/dispatch", "roko-cli/runtime_feedback", "roko-core/config"]
created = 2026-10-15
updated = 2026-10-21
last_verified = 2026-10-21
last_verified_rev = "08c873c71"
source = "status-quo-2026-10-15#finding-17"
anchors = ["crates/roko-cli/src/dispatch/model_routing.rs::compose_propensities", "crates/roko-cli/src/dispatch/model_routing.rs::decide", "crates/roko-cli/src/runtime_feedback/routing.rs::credits_router", "crates/roko-core/src/config/routing.rs::RoutingConfig", "crates/roko-learn/src/cascade_router.rs", "crates/roko-cli/src/graph_execution/learning_commit.rs::commit_run_learning"]
lane = "rust-hot"
parent = "spec-7a71d6"
links = { depends_on = ["gap-b38ed2"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn ladder_rung_exploration_logs_propensity_below_one' crates/roko-cli/ && cargo test -p roko-cli ladder_rung_exploration_logs_propensity_below_one"

[closed]
at = 2026-10-21
at_ts = "2026-10-21T04:45:31Z"
commit = "08c873c71"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-21T01:31:00Z"
forced = false
evidence = "08c873c71, decision 4111 option B1. roko-core: `LadderRung::alternates` (issues for an empty or repeated alternate; the loader's schema sentinel, so `roko config validate` and loads keep it: ladder_survives_a_config_load). roko-cli: on a rung with two or more runnable models `ModelRouter::explore_within_rung` draws among them ε-greedy around the rung's own model (`RoutingLadder::rung_choices`) and returns `ModelChoiceSource::Ladder { rung, draw: Some(RungDraw { propensity, weight }) }`; the route row keeps source `ladder` (no L-route opportunity) and logs chosen_propensity below 1 with every rung model's p, summing to 1, and proposals.ladder = the rung's own model. The routing sink credits `RouterCredit::Weighted(weight)`, weight = (ε/k)/p: a full confidence trial and category count, and a LinUCB update at that share, journaled through `ModelCallJournal::observe_weighted_task_outcome` (plain path `CascadeRouter::record_weighted_outcome`); `credits_router` is true for a draw, so hindsight can retract it. No draw (p = 1, no credit) on a one-model rung, a keyless route, ε 0 or maximize mode, a frozen run (factory `with_rung_exploration(!learning.frozen)`; a frozen run also registers no routing sink and commits nothing), a chain held out on the route layer, or an active self-model's route (`decide_self_model` runs the rung's own model); shadow mode routes by the ladder as before, and a try a draw ran on an alternate is learned by M3 but left out of its calibration window (gap-b0fd8f). Run end: `commit_run_learning` proposes no router version when `CascadeRouter::learned_since_save()` is false and saves unchecked instead, so the file is still written and commits.jsonl gains no row (a_run_that_taught_the_router_nothing_proposes_no_version: three empty runs, 0 versions; one learning run, 1). roko.toml: mid = glm-4-7 with alternates = [kimi-k2-5]; `roko doctor` names alternates apart from rungs. Verify: `cargo test -p roko-cli ladder_rung_exploration_logs_propensity_below_one` passes on 08c873c71 (400 attempts on a two-model rung at ε = 0.1: every row p 0.95 or 0.05, draws on both models, 400 credited confidence trials, the own model's LinUCB arm grows by (ε/2)/0.95 of the alternate's per pick, matched to 1e-9; a one-model rung logs 1.0 and credits nothing; keyless, ε 0, rung draws off and held-out routes draw nothing). Also pass: a_rung_draw_credits_the_router_at_its_weight (both sink paths, and the WAL replay keeps the weight), a_two_model_rung_draws_and_credits_the_router (Graph dispatch with a fake CLI provider; frozen and maximize make no draw and credit nothing), learned_since_save_follows_observations_and_saves. Full lib suites: roko-cli 3965 passed (share_run_scrubs_output_text_and_prompt skipped), roko-core 2046, roko-learn 1392. Integration: tier_ladder_canary, learning_wiring_census, loop_audit_census_run, homeostat_on_mode, learn_paths, learn_paths_fixture, engine_convergence_contract, gate_parity, runner_facades_e2e, dispatch_feedback_projection_e2e pass; the 5 failures (doctor_human_output_succeeds_for_bootstrapped_workspace, phase0_wiring x2, plan_validate x2) are bug-c63cca and bug-2a31bc in a HOME with no claude CLI or keys. cargo +nightly fmt --all --check and clippy -p roko-core -p roko-learn -p roko-cli --no-deps -D warnings clean on 08c873c71. Follow-ups filed: gap-03d8bc, gap-b0fd8f."
+++

## Problem

On default plan runs the cascade router learns nothing. `[routing.ladder]` is on by default
(`crates/roko-core/src/config/routing.rs::RoutingConfig`, `ladder`), the ladder picks every plan task's start rung, and
the router's pick is only logged beside it as a shadow (`crates/roko-cli/src/dispatch/model_routing.rs:21-24`).
Decision 4111 (A) credits the router only for its own picks (`crates/roko-cli/src/runtime_feedback/routing.rs::
credits_router`, :248), and ε-exploration never fires on a ladder rung (`RoutingConfig::explore_epsilon` doc: "a ladder
rung is never explored"). So every L-route row has `chosen_propensity` 1.0.

Seen live (A05 §4 S3, 4114 PARTIAL; E2b L5): tally `cascade-router.json` had `total_observations` = 1 after 10 runs and
28 ladder-routed attempts; in 6 of the 7 real workspaces the ladder made every route.

## Why it matters

Goal `learning` (S02 L1/L3: "the router learns from failures", SC2, which does not hold on default plan runs).
NEW-FINDINGS finding 17. The paper's routing claims (D5; D7 A.4: "No on plan runs: neither acts nor learns") stay as
written (decision 2), so the code has to close the gap. Stage 3 of the epic: measure the router before anything trusts it.

## Where

- `crates/roko-cli/src/dispatch/model_routing.rs` (`decide`, `compose_propensities`, the ladder branch).
- `crates/roko-cli/src/runtime_feedback/routing.rs::credits_router` (decision 4111 (A)).
- `crates/roko-core/src/config/routing.rs::RoutingConfig` (`ladder`, `explore_epsilon`).
- `crates/roko-learn/src/cascade_router.rs` (propensities, 4114).

## Current state

4114's code is done (ε over eligible models with logged propensities, test `route_propensities_sum_to_one_and_respect_
epsilon`); only ladder-free runs exercise it. Decision 4111 said to revisit option (B) once the S04 self-model is
calibrated.

## Plan

Design choice (decision 4111 (B) revisit; under decision 9 of 2026-10-15 the conservative option is recommended):

- **(B1, recommended)** Explore within a rung: when a rung has two or more eligible models, draw among them at ε with
  logged propensities, and credit the router for those picks with IPW weights. A one-model rung is never explored.
  The ladder's climb on failure is unchanged, so quality risk stays within the rung.
- (B2) Also explore one rung down at ε on low-risk task classes. Cheaper but riskier; leave for later.
- (B3) Credit ladder picks to the router as off-policy observations at p = 1. Rejected by 4111 (A) because ladder picks
  are confounded with task difficulty.

Steps for B1: extend the exploration draw to the rung's eligible models; mark the choice source so `credits_router`
accepts it with its propensity; test `ladder_rung_exploration_logs_propensity_below_one` (a two-model rung over many
draws logs propensities below 1 and credits the router; a one-model rung logs 1.0 and credits nothing).

## Done when

The `[[verify]]` command passes, and the workspace's default `roko.toml` ladder has at least one rung with two models,
or the item says why none can.

## Notes

- Depends on gap-b38ed2 (M3 calibration measured), per decision 4111.
- LinUCB stays held (decision 12); this changes only the cascade router's credit.
- 2026-10-16 (learning-research, round v6, at 47b3cdd20): reproduced on fresh workspaces. After three ladder-routed
  plans (7 attempts, 2 climbs), `roko learn route` prints "Cascade router: 0 observations". Every `route_decided` has
  `source: "ladder"`, and every verdict's `ladder.router_pick` is the shadow pick.
  - **Side effect worth fixing with this item:** the run-end guarded commit still writes a new router version every run
    with nothing learned. `roko learn commits` shows router v1→v3 in three runs, each "held-out log-loss 0.6931, the
    last-known-good's 0.6931", with 0 router anchors.
  - **Suggested addition:** skip the router proposal in `graph_execution/learning_commit.rs::commit_run_learning` when
    the run gave the router no credited observation, so the commit log records only real changes.
  - Evidence: `tmp/hermes-roko-v6/learning-research/evidence/zero-default/03-p3-heading-anchors-after/learn-route.txt`
    and `learn-commits.txt`.
