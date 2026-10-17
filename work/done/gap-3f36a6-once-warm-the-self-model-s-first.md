+++
id = "gap-3f36a6"
kind = "gap"
title = "Once warm, the self-model's first-try forecast is confounded by attempt order: on ladder data it forecasts mid at 0.01 although mid passed 20/20, and would start on an untried rung"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "d8943f7e2"
source = "round-v6-learning-research#5"
discovered_from = "agent:learning-research (round v6, 2026-10-17, start-rung demo at b2970cb12)"
anchors = ["crates/roko-learn/src/self_model/model.rs::SelfModel::forecast", "crates/roko-learn/src/self_model/features.rs::TaskFeatures", "crates/roko-learn/src/self_model/logit.rs::LogitForecast", "crates/roko-cli/src/graph_task_dispatch/self_model.rs::task_features"]
lane = "rust-hot"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = ["bug-92cc56", "gap-e464e6", "gap-dd89ff", "gap-51d2ca"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn ladder_ordered_outcomes_do_not_confound_the_start_forecast' crates/roko-learn/ && cargo test -p roko-learn ladder_ordered_outcomes_do_not_confound_the_start_forecast"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T20:35:58Z"
commit = "74946738a"
size = "M"
claimed_at = "2026-10-17T17:08:32Z"
forced = false
evidence = "74946738a, decision A (the per-arm k rule of a7a519393 fixed shadow but not active mode): every first try is forecast by the L0 per-arm rate (basis prior in predictions.jsonl) and L1 forecasts retries and climbs; the L0 lower bound is clamped to its mean. ladder_ordered_outcomes_do_not_confound_the_start_forecast failed first and passes: warm on 40 ladder-ordered outcomes and then 11 starts on mid, P(mid, first try) >= 0.7, P(cheap, first try) <= 0.2, and the start is mid. $0 start-rung re-run (evidence/start-rung/AB): no warm pick of an untried rung (before: task 19 started on strong), the highest forecast of the first try of cheap after its failures is 0.123 (before: 0.92-0.95), every warm first try is on basis prior, and shadow keeps would_choose glm-4.7 through task 20. L1 for first tries waits on an arm-by-order fix (gap-e464e6); the start pick does not use the lessons feature yet. Checks as for bug-92cc56; the [[verify]] passes on d8943f7e2."
+++

## Problem

Once the self-model is warm (30 settled outcomes, when its L1 logit takes over from the per-arm hierarchical prior),
its forecast for a candidate's *first* try is confounded by the attempt-order features. The ladder runs a higher rung
only after failures, so in ladder-routed data the arm and "an earlier try failed / attempt ordinal" move together, and
the logit credits the order instead of the model.

Measured at b2970cb12, $0, scripted models (cheap always fails the task family, mid always passes; lessons off;
`tmp/hermes-roko-v6/learning-research/evidence/start-rung/`; procedure in `tmp/hermes-roko-v6/findings/
learning-research.md` §5):

- **Ladder-only data** (series `shadow` and `active`, 20 tasks, each cheap ✗, cheap ✗, mid ✓):
  - before warm-up, P(verified) for a first try is cheap 0.003 and mid 0.93 by task 10, which is correct, and it
    "would start on" glm-4.7 from task 2;
  - at task 11 (`trained_on` 30) the first-try forecast becomes mid 0.012 and strong 0.000, and it "would start on"
    gpt-5.4-mini, a rung that never ran;
  - for tasks 11–20 the first-try P(mid) stays 0.01–0.19, while the same model's forecast for mid's actual (third)
    tries is 0.81–0.95. Mid passed 20/20.
- **Mirror image** (series `emul`: tasks 1–5 climb as above, tasks 6–30 start on mid by a `rung` hint and pass):
  - from task 21 (`trained_on` 30) the first-try forecast is cheap 0.943 / mid 0.950 / strong 0.950;
  - cheap failed 10/10. The logit now credits "first try, no earlier failure" with success;
  - it still picks mid, but only by 0.007.
- **The calibration gate can't see either.** It scores only the arm that ran at each attempt (cheap on tries 1–2, mid
  on try 3), and those forecasts are sharp. So ECE is 0.000–0.05, Brier skill 0.97–1.0 and AUROC 1.0 throughout,
  while the counterfactual first-try forecasts that drive the start-rung choice are wrong.

## Why it matters

Goal `cybernetic`; the round-v6 pitch's "learns where to start".
- Once bug-92cc56 lets active mode act, a self-model trained on ladder data would start tasks on the wrong rung (here
  the untried, priciest one) as soon as it is warm.
- Its own mid-first starts during the warm-up window may break the confound. The `emul` series suggests so, but only
  narrowly.
- In shadow mode, which is what the live showcase plans to show, `route_decided.forecast` (`would_choose`, `p_vs`)
  is wrong in any warm workspace whose data the ladder produced, and nothing in the event says the model is warm.

Related:
- gap-e464e6 (warm forecasts swing; a different mechanism, the step size);
- bug-92cc56 (active can't act);
- gap-dd89ff (no knowledge feature, another missing confounder);
- gap-51d2ca (rung exploration).

## Where

- `crates/roko-learn/src/self_model/model.rs::SelfModel::forecast`: blends the L0 prior and, once `outcomes >= N_MIN`
  (30), the L1 logit.
- `crates/roko-learn/src/self_model/features.rs::TaskFeatures`: the attempt ordinal and earlier-failure features
  beside the arm one-hots.
- `crates/roko-learn/src/self_model/logit.rs::LogitForecast`: the per-feature online update.
- `crates/roko-cli/src/graph_task_dispatch/self_model.rs::task_features`: builds the features for each candidate at
  forecast time.

## Current state

The L0 prior (per model, family and role) forecasts correctly in this setting. The switch to L1 at 30 outcomes is
where it breaks. No exploration of start rungs happens in shadow (ε only explores router or active self-model routes),
so ladder data never contains a higher rung on a first try.

## Plan

Options, not exclusive:
1. Forecast the start choice from features that ladder order can't confound: drop the attempt-order features from the
   arm-choice forecast, or fit arm × attempt interactions only where both orders have data. Keep the order features
   for the retry/climb decision, where they belong.
2. Keep L0 as the shrinkage target: blend L1 toward the per-arm L0 rate by the support each arm has on a first try
   (zero for a rung that never started a task), instead of switching at a global `N_MIN`.
3. Explore the start rung at a small ε even in shadow's data plan (a logged-propensity draw that starts some tasks one
   rung up), so first-try data exists for every rung (gap-51d2ca B1/B2 for M3).
4. Put `trained_on` (or the forecasting layer, L0/L1) on `route_decided.forecast`, so a UI can say how much the
   forecast rests on.

Then test `ladder_ordered_outcomes_do_not_confound_the_start_forecast`:
- train on 40 ladder-ordered outcomes (cheap ✗ at tries 1–2, mid ✓ at try 3);
- the first-try forecast keeps P(mid) ≥ 0.7 and P(cheap) ≤ 0.2;
- the pick is mid, never an unobserved rung.

## Done when

The `[[verify]]` command passes, and re-running the round-v6 `shadow` series
(`tmp/hermes-roko-v6/learning-research/scripts/rung_demo.sh`, 20 tasks) keeps "would start on" at glm-4.7 through
task 20.

## Notes

2026-10-17 (roko-recovery, the lead's decisions):
- **The first fix was not enough.** It forecast an arm's first try from L0 only while the arm had fewer than 5
  learned first tries. That fixed shadow mode (would_choose stayed glm-4.7 through task 20), but not active mode,
  in learning-research's re-run (`tmp/hermes-roko-v6/learning-research/evidence/start-rung/merged/`):
  - at task 19, the first warm task, mid had 11 learned first tries, so L1 forecast it; L1 dipped mid's first try
    to 0.11–0.32 and the pick went to the untried strong rung;
  - from task 20, L1 forecast cheap's first try at 0.92–0.95 after 7 straight first-try failures.

  L1 at about 30 online outcomes cannot forecast a first try. The order features are shared across arms, and its
  warm forecast swings (gap-e464e6).
- **Decision A, now the fix.** Every first try is forecast by L0's per-arm rate (`basis: "prior"` in
  `predictions.jsonl`); L1 forecasts retries and climbs.
- **Left for later:**
  - L1 for first tries waits on an arm-by-order fix (gap-e464e6);
  - until then, a first try's start pick does not use L1's task features, gap-dd89ff's lessons feature included.
