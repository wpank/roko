+++
id = "gap-e464e6"
kind = "gap"
title = "L1 self-model forecasts swing 0.03-0.97 for a mixed pass rate instead of settling, once warm"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-learn/self-model"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-9 follow-up reports 2026-10-03 (PK52 gap-0c429f)"
discovered_from = "gap-0c429f"
anchors = ["crates/roko-learn/src/self_model/logit.rs::LogitForecast", "crates/roko-learn/src/self_model/model.rs::N_MIN"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn forecast_settles_at_a_fixed_mixed_pass_rate' crates/roko-learn/ && cargo test -p roko-learn forecast_settles_at_a_fixed_mixed_pass_rate"
+++

## Problem

Once the self-model is warm (`outcomes >= N_MIN`, `crates/roko-learn/src/self_model/model.rs:33,116`, `N_MIN = 30`),
its L1 logit forecasts don't settle for a rung with a mixed pass rate. `LogitForecast::update`
(`crates/roko-learn/src/self_model/logit.rs:108-115`):

```rust
let h = self.curvature.entry(name.to_string()).or_insert(0.0);
*h += w * p * (1.0 - p) * value * value;
let step = w * error * value / (*h + LAMBDA2);
*self.weights.entry(name.to_string()).or_insert(0.0) -= step;
```

With roughly 24 always-on features each taking the full `step` on every outcome, and `h` (the curvature/learning
-rate denominator) growing only by `p(1-p)` per update, one outcome moves the aggregate score by about 6 in
log-odds — enough to swing `sigmoid(score)` between 0.03 and 0.97 on consecutive updates, oscillating rather than
converging toward the rung's true pass rate. Nothing in `update` scales the step by the *score's* variance
(`v = Σ x_j²/(h_j+λ2)`, already computed elsewhere in this file for the forecast's own shrinkage term,
`logit.rs:~100`, `(1.0 + π*v/8).sqrt()`) — only each *feature's* own curvature, which several always-on features
sharing credit for the same outcome doesn't account for.

## Why it matters

Goal: cybernetic, M3 self-model (S04). A forecast that swings 0.03-0.97 instead of settling means the
calibration gate (`CalibrationGate::evaluate`, ECE/calibration-in-the-large/BSS/AUROC bounds) would, in practice,
never see it as calibrated — this keeps active mode off indefinitely once the model is warm, defeating the whole
point of having a self-model that's supposed to graduate out of shadow mode.

## Where

- `crates/roko-learn/src/self_model/logit.rs::LogitForecast::update` (the step/curvature computation) and
  `::score`/the `v` computation (the already-available score-variance term this update doesn't use).
- `crates/roko-learn/src/self_model/model.rs::N_MIN` (the warm-up threshold at which this becomes observable).

## Current state

Confirmed as described: no damping term in `update` beyond each feature's own `h`.

## Plan

Two options, as reported:

1. **Scale the update by the score's variance**: something like dividing the step by `(1 + v*p*(1-p))` (using
   the same `v` the forecast's shrinkage term already computes), so a shared outcome's effect on each always-on
   feature shrinks as the aggregate score's uncertainty shrinks.
2. **Damp shared always-on features specifically**: give always-on features their own, faster-growing curvature
   (e.g. a shared pseudo-count), so they individually converge faster and stop contributing large steps once
   well-estimated, without changing every feature's update rule.

Either needs a test simulating many outcomes at a fixed mixed pass rate (e.g. p=0.6) and asserting the forecast's
variance across the tail of updates stays bounded (doesn't keep swinging 0.03-0.97).

## Done when

- At a fixed, mixed true pass rate, repeated outcomes make the L1 forecast converge and stay near that rate,
  not oscillate between near-0 and near-1.
- The `[[verify]]` command passes.

## Notes

- Severity p2, per the report's own framing (a design issue for S04/M3, not yet a committed fix — the two
  options above are suggestions, not a decided approach).
- Discovered during PK52's work (gap-0c429f, done).
