+++
id = "bug-09173e"
kind = "bug"
title = "The self-model forecasts cost and latency q90 of 6e26 for an arm it has never observed (still $99 and 15.6 h after 6 outcomes)"
status = "done"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/self_model"]
created = 2026-10-16
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "14e76b409"
source = "round-v6-learning-research#4a.3"
discovered_from = "agent:learning-research (round v6, 2026-10-16)"
anchors = ["crates/roko-learn/src/self_model/cost.rs::CostModel::forecast", "crates/roko-learn/src/self_model/cost.rs::StudentT"]
lane = "rust-cold"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = ["gap-b38ed2"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn cost_forecast_q90_is_bounded_without_observations' crates/roko-learn/ && cargo test -p roko-learn cost_forecast_q90_is_bounded_without_observations"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T04:20:30Z"
commit = "14e76b409"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-17T02:10:46Z"
forced = false
evidence = "14e76b409 takes the item's plan 1. roko-learn self_model/cost.rs::CostModel::forecast(arm, x, prior_cost_usd) forecasts from a regression only once it has observed the arm (its one-hot set by rows of positive weight); otherwise the quantiles come from a log-normal prior of spread PRIOR_LOG_SD = 1.1 (central 90% about 37x, bai2026spend's 30x; q90 = 4.1x the median): cost around the price snapshot's price of a TYPICAL_ATTEMPT on the model (6k input, 24k cache-read and 3k output tokens, the round-v6 live median rounded), latency around the observed attempts' geometric mean, else PRIOR_LATENCY_S = 60 s. Latency uses the observed level, not a provider class, because the cost model only knows the arm's provider key. self_model/model.rs::SelfModel prices that attempt on each snapshot model, with the snapshot median ($0.035 at prices-2026-09-28) for an unpriced model, rebuilt from the snapshot on load and never saved, so the state file and predictor version are unchanged. CostForecast, CandidateForecast and the roko.prediction/1 PredictionCandidate carry `cost_prior` and `latency_prior` (serde default false), so a row says when its costs are a prior; an observed arm's q50 and q90 come from the regression as before. The literal verify, `grep -rqw 'fn cost_forecast_q90_is_bounded_without_observations' crates/roko-learn/ && cargo test -p roko-learn cost_forecast_q90_is_bounded_without_observations`, passes at 14e76b409 (11 test binaries, the named test ok): on a fresh model gpt-oss-120b, glm-4.7 and gpt-5.4-mini each forecast their typical attempt's price ($0.0128, $0.0128, $0.0198) with a finite cost q90 under ten times it and a latency q90 under ten minutes, both flagged; after the evidence's six outcomes on the first two, those forecast from what they cost, and gpt-5.4-mini still reads as its price, with the six attempts' 12 s geometric-mean latency. A fresh workspace's first predictions.jsonl row is checked in default_plan_run_forecasts_in_shadow (run_graph_plan, the function `roko plan run` calls, in a new temp workspace) and in self_model_shadow_writes_predictions_and_keeps_routing: every candidate is flagged as priors, cost q90 under $1, latency q90 under 600 s. Also passing at 14e76b409: roko-learn `cargo test -p roko-learn -- self_model telemetry` over all targets (106, among them an_unobserved_arm_forecasts_its_prior), and roko-cli over all 76 targets: the 17 tests matching self_model, plan_run_writes_calibration_json, default_plan_run_forecasts_in_shadow, default_plan_run_registers_m1_in_shadow and the 5 learn_self_model tests. `cargo clippy -p roko-learn -p roko-core -p roko-cli --no-deps -- -D warnings` is clean, and the branch is formatted with `cargo +nightly fmt --all`. Filed bug-f6682e: an observed arm on a task family the cost model has not seen still explodes (measured: cost q50 $0.072 / q90 $472 and latency 10 s / 8,618 s, against $0.0195 / $0.039 and 31 s / 54 s on its own family)."
+++

## Problem

The self-model's cost and latency forecasts for an arm it has never observed are absurd. They are `exp()` of a vague
Student-t quantile on log cost and log seconds:

- run 1 of a fresh workspace (n = 0): every arm `cost_q90 = 6.08e26` USD and `lat_q90_s = 6.08e26`;
- after 6 outcomes, the never-run rung gpt-5.4-mini: `cost_q90 = 99.01` USD and `lat_q90_s = 56166` (15.6 hours).
  The arms that ran were sane: gpt-oss-120b $0.045 / 18 s and glm-4.7 $0.017 / 131 s.

Evidence: `tmp/hermes-roko-v6/learning-research/evidence/live/workspace/runs/*/predictions.jsonl` and
`evidence/zero-default/workspace/runs/*/predictions.jsonl` (round v6, `findings/learning-research.md` §4a.3).

## Why it matters

Goal `cybernetic`. In shadow these numbers only sit in `predictions.jsonl`, but they reach decisions:
- policy (a) (`lcb_aci`) and S04's cost-aware choices read `cost_q90`;
- any UI that shows the forecast will print them.

An unobserved arm should read as "unknown, probably like its price tier", not as $10^26.

## Where

- `crates/roko-learn/src/self_model/cost.rs::CostModel::forecast`: `cost_q90: cost.quantile(0.9).exp()` and
  `lat_q90_s: latency.quantile(0.9).exp()`.
- `crates/roko-learn/src/self_model/cost.rs::StudentT`: the predictive with few degrees of freedom and a large scale.

## Current state

The NIG regression's prior is wide on purpose. The blow-up is in exponentiating its upper quantile with almost no
degrees of freedom.

## Plan

1. Bound the forecast by a prior anchored on the price snapshot: an unobserved arm's cost quantiles come from the
   snapshot's per-token rates times the role's typical tokens, and latency from the provider class, until the model has
   observations of that arm. Alternatively, clamp the exponent to a documented ceiling and flag the forecast
   `prior_only`.
2. Test `cost_forecast_q90_is_bounded_without_observations`: with no observations, q90 is finite, under a stated
   ceiling (for example 10× the snapshot price of a typical attempt), and the row says it is a prior.

## Done when

The `[[verify]]` command passes, and a fresh workspace's first `predictions.jsonl` row has finite, plausible q90 values.

## Notes

Don't change the q50 path for observed arms; their forecasts in the evidence look right.
