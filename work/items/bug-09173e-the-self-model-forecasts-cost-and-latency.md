+++
id = "bug-09173e"
kind = "bug"
title = "The self-model forecasts cost and latency q90 of 6e26 for an arm it has never observed (still $99 and 15.6 h after 6 outcomes)"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/self_model"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "round-v6-learning-research#4a.3"
discovered_from = "agent:learning-research (round v6, 2026-10-16)"
anchors = ["crates/roko-learn/src/self_model/cost.rs::CostModel::forecast", "crates/roko-learn/src/self_model/cost.rs::StudentT"]
lane = "rust-cold"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = ["gap-b38ed2"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn cost_forecast_q90_is_bounded_without_observations' crates/roko-learn/ && cargo test -p roko-learn cost_forecast_q90_is_bounded_without_observations"
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
