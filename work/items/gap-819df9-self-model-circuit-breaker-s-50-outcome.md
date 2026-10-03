+++
id = "gap-819df9"
kind = "gap"
title = "Self-model circuit breaker's 50-outcome/10-bin ECE check will trip on sampling noise, not just drift"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-learn/self-model"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-7 follow-up reports 2026-10-03 (PK48 gap-d90ef6)"
discovered_from = "gap-d90ef6, design finding for S04"
anchors = ["crates/roko-learn/src/self_model/gate.rs::breaker_tripped", "crates/roko-learn/src/self_model/metrics.rs::ECE_BINS"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn breaker_false_positive_rate_is_bounded_for_a_calibrated_model' crates/roko-learn/ && cargo test -p roko-learn breaker_false_positive_rate_is_bounded_for_a_calibrated_model"
+++

## Problem

The self-model's circuit breaker (`crates/roko-learn/src/self_model/gate.rs::breaker_tripped`, S04 §4.11.5) trips
an active predictor back to shadow mode when, over the last `BREAKER_WINDOW = 50` outcomes, either the Brier
score is worse than the base rate's, or ECE exceeds `BREAKER_ECE = 0.2` (computed via `ece(&recent, ECE_BINS)`
with `ECE_BINS = 10`, `metrics.rs:15`).

10 equal-mass bins over 50 outcomes puts 5 outcomes in each bin. A perfectly calibrated model's *expected* ECE at
that bin size is not 0 — the per-bin estimation noise alone (E|Binomial(5, p)/5 − p|) is already large relative to
the 0.2 threshold:

- p = 0.5: E|Bin(5, 0.5)/5 − 0.5| ≈ 0.19
- p = 0.7: ≈ 0.17
- p = 0.9: ≈ 0.12

So a model that is genuinely well-calibrated can show an *expected* ECE close to the 0.2 trip threshold from
sampling noise alone, before any real miscalibration. The breaker will fire on noise, not drift, more often than
S04 §4.11.5 intends, dropping an active model to shadow when nothing is actually wrong with it.

## Why it matters

Goal: cybernetic, M3 self-model (S04). A breaker that fires on sampling noise defeats its own purpose — it should
distinguish "this model's calibration degraded" from "this window happened to land on the high side of a
perfectly calibrated distribution." Every spurious trip costs a demotion-and-recovery cycle and erodes trust in
the breaker's signal (if it fires too often on good models, operators will learn to ignore it, which is worse
than not having it).

## Where

`crates/roko-learn/src/self_model/gate.rs`: `BREAKER_WINDOW` (50), `BREAKER_ECE` (0.2), `breaker_tripped`
(~line 216). `ECE_BINS` (10) lives in `crates/roko-learn/src/self_model/metrics.rs:15` and is shared with the
promotion gate's own `ECE_MAX = 0.08` check (`CalibrationGate::evaluate`, `WINDOW = 100`) — note the promotion
gate uses a *larger* window (100) and a *tighter* bound (0.08), which is the opposite problem (its own noise
floor at n=100/10 bins = 10/bin is smaller, so 0.08 is more defensible there); this item is about the *breaker*
specifically, at its smaller 50-outcome window.

## Current state

Unfixed; this is a design/statistics finding from PK48's work on the self-model replay (gap-d90ef6, done), not yet
reduced to a recommended fix. Options, as reported:

1. **Fewer bins for the breaker specifically** (e.g. 4-5 bins instead of 10 at the same 50-outcome window) —
   raises outcomes-per-bin, lowering the noise floor, with less resolution on where miscalibration occurs.
2. **A longer window** for the breaker (closer to the promotion gate's 100) — more outcomes per bin at the same
   bin count, but slower to react to a real regression, which works against the breaker's purpose (catching
   drift quickly).
3. **A debiased ECE estimator** (e.g. a bias-corrected or binless estimator) instead of raising bin size or
   window — keeps responsiveness and resolution, more statistical work to implement and verify.

## Plan

This item is a design question for S04/S09, not yet a committed fix — the "Done when" below is "a decision is
recorded and implemented," not a specific code change, since the right option depends on a trade-off (reaction
speed vs. false-positive rate) that needs Will's or S04's call, similar to how other PK48 assumptions (see
gap-997366-adjacent work and this wave's entry 4) get confirmed with S04/S09 before implementation.

1. Decide among the three options above (or propose a fourth) — record the decision analogous to how `DECISIONS.md`
   entries work for other S04/S09 defaults.
2. Implement the chosen fix in `breaker_tripped`, with a test asserting the breaker's false-positive rate at the
   new configuration stays acceptably low for a simulated perfectly-calibrated model over many windows (a Monte
   Carlo test, not just a single fixed-seed example, since the whole point is a false-positive *rate*).

## Done when

- A decision is recorded on which mitigation to use.
- `breaker_tripped`'s false-positive rate for a well-calibrated model is measured (not just asserted by
  construction) and is acceptably low at the new configuration.
- The `[[verify]]` command passes.

## Notes

- Severity p2, per the wave-7 report framing ("design, for S04").
- The arithmetic above (0.19/0.17/0.12) is the *expected absolute* per-bin deviation for a binomial count at
  n=5, not a worst-case bound — real ECE also depends on how predictions are distributed across bins, so actual
  false-positive rates should be measured empirically (Monte Carlo), not assumed from this back-of-envelope figure
  alone, when choosing and verifying a fix.
