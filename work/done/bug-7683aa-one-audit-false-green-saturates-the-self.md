+++
id = "bug-7683aa"
kind = "bug"
title = "One audit false green saturates the self-model's false-green head: a VS 0 label at weight 1/π = 20 moves OnlineLogit by about 9.7 per feature in one step, so once warm p_fg is 1.0 for every arm and P(VS) is 0 everywhere (start-rung demo v7-after-ad16cc844, task 13 on)"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/self_model"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
last_verified_rev = "d1758ed63"
source = "round-v7-roko-speed#C-demo"
discovered_from = "agent:roko-speed (round v7, gap-74893d's start-rung demo after-run)"
anchors = ["crates/roko-learn/src/self_model/logit.rs::OnlineLogit", "crates/roko-learn/src/self_model/model.rs::forecast_arm"]
links = { depends_on = [], blocks = [], related = ["gap-7c041c", "gap-74893d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn one_audit_false_green_does_not_saturate_the_false_green_head' crates/roko-learn/src && cargo test -p roko-learn --lib one_audit_false_green_does_not_saturate_the_false_green_head"

[closed]
at = 2026-10-20
at_ts = "2026-10-20T21:20:26Z"
commit = "d1758ed63"
by = "roko-speed"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-20T19:23:20Z"
forced = false
evidence = "d1758ed63. The false-green head is now FalseGreenHead (self_model/false_green.rs): a Beta-binomial rate with a prior worth N = 400 pseudo-passes at 5% (FALSE_GREEN_STRENGTH = 1/(π_floor·p0), π_floor = S05's 0.05 audit floor, roko_gate EPS_FLOOR), a global rate and one per model shrunk towards it, counts not discounted. SelfModel learns it from VS-labelled passes (observe_with, observe_vs) and reads it once warm (forecast_arm); L1's GateForecaster keeps the per-rung heads only; a state saved before loads with a fresh head. The literal verify passes on the branch: one_audit_false_green_does_not_saturate_the_false_green_head (warm model, one VS 0 audit at weight 20: the labelled arm's p_fg about 0.138, an unlabelled arm's 0.095, P(VS) above 0.8·p_gate; 200 more converge above 0.95); also the_head_moves_by_the_weight_against_its_prior and the existing late-VS test. Full lib suites: roko-core 2044, roko-learn 1390, roko-serve 1073, roko-cli 3937 with one failure, bug-e362a1's known heartbeat flake, which passes alone; learn_self_model bin tests 5; fmt and clippy clean. Start-rung demo on the fixed binary (tmp/hermes-roko-v6/learning-research/evidence/start-rung/v7-fg-*): six draws with explore_epsilon 0 all route every warm task on mid (24 of 24) with the false-green head peaking at 0.138-0.282 after one to three definite VS 0 audits (the old head went to 1.0 and sent every task to strong); the six lessons-off conditions pass in four draws, and the two misses are the cost condition over tasks 6-20 at 59.3% and 59.6% against its 60% bar, with identical routing, while over tasks 7-20 all six are 63.6-65.3% (the emulation's ceiling is 64.7%). Two draws with roko's default exploration: 6 of 6 (-61%), and 5 of 6 (-47%) after an exploration draw on cheap at task 12 dropped the gate for two tasks."
+++

## Problem

One audit label that finds a false green drives the self-model's false-green head to 1.0 for every arm. Once the
model is warm, that sets P(VS) to 0 everywhere.

Seen in round v7's start-rung demo (`rung_rerun.sh`, tag `v7-after-ad16cc844`, series `fix-active`, scripted models,
$0):
- At task 9, S05's audit (π = 0.05, so weight 1/π = 20) ran a hidden suite on mid's pass. The suite has one test, and
  it failed: `vs: 0`, `unknown: false` in `labels.jsonl` of run `graph-r09-slug-v9-…`.
- From task 13 (`trained_on_n` 32, past N_MIN = 30), every candidate's forecast has `p_fg` = 1.0, so `p_vs` and
  `p_vs_lcb` are 0 for cheap, mid and strong alike. Mid's `p_gate` is still 0.986, and mid passes every task.
- The run before it on a1d12fc1c drew two audits, both `unknown` (no hidden suite), so `p_fg` stayed at its 0.05
  prior.

The cause is `OnlineLogit::update` in `logit.rs`. Its step is w·(p − y)·x/(h + λ2), and a fresh head has curvature
h ≈ 0 against λ2 = 1. With w = 20, p ≈ 0.05 and y = 1, each one-hot feature's weight moves by about 9.7 in one update.
Every task shares those features (the bias, tier, family and role), so the score jumps from logit(0.05) = −2.9 to far
above 0. While cold, the forecast uses the 0.05 prior (`SelfModel::forecast_arm`), so the saturation only shows once
the model is warm.

## Why it matters

With P(VS) at 0 everywhere:
- the cascade's expected totals are all infinite, and the break-even rule clears no rung, so both start rules fall
  back to the last rung (in the demo, strong from task 13 on);
- `after_failure` and policy (a) see every rung below `p_abandon` (0.15) and would abandon;
- the shadow advice is meaningless until enough VS-verified passes pull the head back.

Audits are rare by design (π = 0.05), so each label carries weight 20. That makes this the expected outcome of the
first false green an audit finds, not an edge case.

## Where

- `crates/roko-learn/src/self_model/logit.rs`: `OnlineLogit::update` (the step), `GateForecaster::observe` and
  `observe_vs` (the false-green head's updates), `LAMBDA2`.
- `crates/roko-learn/src/self_model/model.rs`: `forecast_arm`, which switches from `FALSE_GREEN_PRIOR` to the head
  once warm.

## Plan

Options (choose with the fix):
1. Bound one update's step (a trust region on the change in score), or split a weight-w update into steps.
2. Give the false-green head a prior worth N pseudo-outcomes at 0.05 (a larger λ2 for this head), so one weighted
   label moves it by about w/(w + N).
3. Pool the head per arm and shrink it toward the prior like L0's strata.

## Done when

A warm model that learns one audit false green at weight 20 forecasts a `p_fg` well below 1, and above the prior, for
a different task. The demo's `fix-active` series keeps P(VS) for mid near its pass rate after such a label. Checked
by the test in `[[verify]]`.

## Notes

- Found by roko-speed while running gap-74893d's before/after demo (round v7). The start rule did not cause it: on all
  50 first tries of the after run's two series, both rules picked the same rung except on task 1, where the model had
  no data and the gate was shut.
- Related: gap-7c041c (once warm, L1's first p_gate forecasts saturate the same way).
