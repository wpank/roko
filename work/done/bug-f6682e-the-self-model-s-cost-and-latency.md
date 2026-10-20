+++
id = "bug-f6682e"
kind = "bug"
title = "The self-model's cost and latency q90 still explode for an observed arm on a task family the cost model has not seen: q90 $472 for an arm that costs $0.02"
status = "done"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/self_model"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "323524a6a"
source = "bug-09173e worker, 2026-10-17: a CostModel probe after its fix, 20 focused attempts then one integrative forecast"
discovered_from = "bug-09173e"
anchors = ["crates/roko-learn/src/self_model/cost.rs::CostModel::forecast", "crates/roko-learn/src/self_model/cost.rs::NigRegression::predictive", "crates/roko-learn/src/self_model/model.rs::cost_features"]
lane = "rust-cold"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = ["bug-09173e", "gap-e464e6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn cost_forecast_q90_is_bounded_on_an_unseen_family' crates/roko-learn/ && cargo test -p roko-learn cost_forecast_q90_is_bounded_on_an_unseen_family"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T14:45:08Z"
commit = "323524a6a"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-17T11:04:28Z"
forced = false
evidence = "323524a6a takes the item's plan 1, generalized from 'features not observed' to the rows' support. roko-learn self_model/cost.rs: NigRegression::support(x) = 1/(x'Lambda^-1 x), the attempts' worth of data the posterior location at x rests on (a feature with no dimension adds x^2/kappa), and CostModel::forecast uses a regression's Student-t predictive only at support >= MIN_SUPPORT = 0.25. Below it the cost and latency quantiles are bug-09173e's log-normal prior (PRIOR_LOG_SD = 1.1, q90 = 4.1 x q50), flagged cost_prior/latency_prior, centred on the arm's geometric-mean target over its rows (xty[arm]/xtx[arm][arm], already in the sufficient statistics, so no saved state, feature or prior changes and the predictor version stays); an arm with no rows keeps the snapshot price and the observed attempts' latency. Measured supports: an arm's own stratum about its row count (20 for the item's case), a family tied to the arm through another arm's rows about 1 (that forecast stays the regression's), the item's unseen family 0.006, a family only another arm ran 0.01 (q90 376 x q50 before, though every feature was observed, which the literal plan-1 check would have missed). The literal verify, `grep -rqw 'fn cost_forecast_q90_is_bounded_on_an_unseen_family' crates/roko-learn/ && cargo test -p roko-learn cost_forecast_q90_is_bounded_on_an_unseen_family`, passes at 323524a6a (11 test binaries, the named test ok): after 20 focused attempts of gpt-oss-120b at about $0.02 (log sd 0.5) and 30 s (log sd 0.3), its integrative forecast is flagged, at the geometric mean of its costs and latencies, q90 finite and at most 10 x q50, q50 within a factor of two of $0.02 and 30 s, and its focused forecast is still the regression's. The same test fails with main's cost.rs (cost q50 $0.072 / q90 $1,969, latency 9.5 s / 6,397 s, unflagged). Also passing: all 61 roko-learn `self_model::` lib tests, among them the new an_unsupported_forecast_is_anchored_on_the_arm (the cross case, the tie through one row, a spec-size extrapolation, and an observed stratum equal to the regression's predictive) and bug-09173e's cost_forecast_q90_is_bounded_without_observations and an_unobserved_arm_forecasts_its_prior, unchanged. `cargo clippy -p roko-learn --no-deps -- -D warnings` is clean and `cargo +nightly fmt -p roko-learn` was applied, both through the limiter."
+++

## Problem

The self-model's cost and latency q90 still explode for an arm it has observed, as soon as a task's family is one
the cost model has not seen. Plan tasks use their tier as the family (`mechanical`, `focused`, `integrative`,
`architectural`), so a workspace that has run only focused tasks forecasts its first integrative task with absurd
upper quantiles on every rung.

Measured with `CostModel` (2026-10-17, at bug-09173e's fix): after 20 attempts of gpt-oss-120b on focused tasks, at
about $0.02 (log sd 0.5) and 30 s (log sd 0.3), its focused forecast is cost q50 $0.0195 / q90 $0.039 and latency
31 s / 54 s, while its integrative forecast is cost q50 $0.072 / q90 $472 and latency 10 s / 8,618 s.

The cost and latency regressions (`NigRegression`) have the features [1, arm one-hot, log(spec_tokens), family
one-hot, n_declared_paths] with a vague prior, κ = 0.01 on every coefficient. When every observed row has the same
family, the bias, the arm and the family one-hot are collinear, and the ridge posterior splits the arm's log target ȳ
evenly between them. For the arm on a new family:

- the location loses the observed family's share, about 2ȳ/3, so the median is about exp(−ȳ/3) off: 3.7 times too
  high for a $0.02 cost, three times too low for a 30 s latency;
- the variance gains about (2/3)/κ ≈ 67 from the split and 1/κ = 100 from the new family's coefficient, so q90 runs
  to thousands of times q50.

bug-09173e bounded the case of an arm with no observations of its own (its forecast is a price-anchored prior, flagged
`cost_prior`/`latency_prior`); this is the same blow-up for an observed arm.

## Why it matters

Goal `cybernetic`. Policy (a) filters arms by `cost_q90 ≤ B_remaining`, and both policies take `expected_cost`, which
turns q90/q50 into exp(σ²/2): a new family makes every rung look unaffordable or infinitely dear, and the start rung
and post-pass loss L_fg go with it. Any UI showing the forecast prints it.

## Where

- `crates/roko-learn/src/self_model/cost.rs::CostModel::forecast` and `NigRegression::predictive`: the predictive
  over a feature the regression has never set, and the collinear one-hots.
- `crates/roko-learn/src/self_model/model.rs::cost_features`: the features.

## Current state

Open. Found while doing bug-09173e (2026-10-17), whose fix covers unobserved arms only.

## Plan

Options:

1. Treat a forecast whose features the regression has not all observed as a prior, as bug-09173e does for an
   unobserved arm, anchored on the arm's own observed level where it has one (its mean log cost over its rows, which
   needs a per-arm running mean) and on the price otherwise.
2. Give one-hot coefficients a proper hierarchical prior (a shared variance learned across arms and families), so a
   new category inherits a learned spread instead of 1/κ, and drop the bias's collinearity (encode one-hots as
   deviations, or drop the bias).
3. Cap the variance an unseen feature adds at a documented prior spread, keeping the location.

Option 2 is the principled fix and changes the observed path; option 1 is the smallest. The predictor version hashes
the feature schema, so a change to the features or the prior needs a schema bump.

## Done when

`cost_forecast_q90_is_bounded_on_an_unseen_family` (the `[[verify]]` command) passes: after 20 observed attempts of
an arm on one family, its forecast on another family has a finite q90 within ten times its q50, and a q50 within a
factor of two of what it costs.

## Notes

Don't change the forecasts of an observed arm on an observed family. The latency regression has the same features and
the same problem.
