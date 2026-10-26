+++
id = "bug-bce0ed"
kind = "bug"
title = "The self-model's retry forecasts saturate and its isotonic map returns exactly 0, so the shadow model proposed abandoning a task its next try passed (phase 4.2 run 1447385e T1#2: p_vs 0.0 on both arms, then a pass on gpt-5.4)"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-learn/self_model"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7-pilot#P-1447385e-T1-2"
discovered_from = "agent:hermes-pilot (round v7 phase 4.2 run 1447385e, findings §P); diagnosed by agent:roko-ladder from Undertow's self-model state 2026-10-26"
anchors = ["crates/roko-learn/src/self_model/logit.rs::update", "crates/roko-learn/src/self_model/recal.rs::apply", "crates/roko-learn/src/self_model/model.rs::forecast_arm", "crates/roko-cli/src/graph_task_dispatch/self_model.rs"]
links = { depends_on = [], blocks = [], related = ["bug-7683aa", "gap-3f36a6", "bug-a35286"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_warm_model_never_forecasts_a_retry_at_exactly_zero' crates/roko-learn/src && cargo test -p roko-learn --lib -- a_warm_model_never_forecasts_a_retry_at_exactly_zero"

[[verify]]
command = "grep -rqw 'fn a_shadow_abandon_says_it_is_advice' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_shadow_abandon_says_it_is_advice"
+++

## Problem

Phase 4.2's run `1447385e` (Undertow, roko main d286361c4) hit this at T1#2, at 11:14:06Z. The shadow self-model
`m3-l1-411ad1f4`, trained on 64 rows, logged:

```
WARN roko_cli::graph_task_dispatch::self_model: self-model: T1 is unlikely to pass on any rung; the attempt runs on
the ladder's choice ... action=abandon spec_score=Some(0.9579) best_p_vs=0.0
```

- **The forecast:** both arms, gpt-5.4-mini and gpt-5.4, had `p_gate`, `p_vs` and `p_vs_lcb` of **exactly 0.0**, on
  basis `logit`. `would_choose` was null. The prediction is copied to
  `tmp/hermes-roko-v7/pilot/evidence/p4-2-t1-1/predictions-1447385e.json`.
- **What happened:** the try ran on the ladder's choice and passed on gpt-5.4: 59 turns, 4/4 checks, $1.03.
- **The prior's view:** T1#1's forecast for strong was 0.622 (lcb 0.48), and T2#1 and T3#1 were 0.51–0.72. All
  three were on the prior basis.
- **The risk:** in active mode this would abandon a task its next try passes, leaving T3 unrun and the phase
  undelivered.

### Why it is exactly 0.0

The model state (`~/dev/undertow/.roko/learn/self-model/state-v1.json`, read 2026-10-26) shows two causes.

1. **L1's raw forecasts saturate.**
   - Of the 66 raw forecasts in the recalibration window, 57 are below 1e-6 or above 1 − 1e-6. 17 are confidently
     wrong: 8.8e-68, 3.6e-32 and 7e-14 for passes; 1.0 for failures.
   - L1 has 794 weights, mostly hashed `bow:` features. Many sit at ±2.0 with curvature `h` around 1e-6.
   - The cause is the online diagonal-Laplace update (`OnlineLogit::update`, λ2 = 1). Each feature steps
     `error·x_j/(h_j + λ2)` independently, so one update moves the score by about `error · Σ x_j²/(h_j + λ2)`, the
     variance `v`. With dozens of fresh active features that is dozens of logits per update.
   - The model overshoots into saturation, `p(1 − p)` then stops `h` from growing, and the next update overshoots
     back.
   - So λ2 alone does not regularize it.
2. **The isotonic recalibration returns exactly 0.**
   - Both folds' maps were accepted, since they beat the saturated raw forecasts' Brier score.
   - Their lowest blocks are all failures, with knots such as (8.4e-35, 0.0) and (6.0e-34, 0.0).
   - `IsotonicMap::apply` returns the block mean unsmoothed, so a raw forecast in that block becomes exactly 0.0, and
     `p_vs_lcb = min(lcb, 0)` is 0.

This is not bug-7683aa's audit false-green zeroing: `p_fg` is the 0.05 prior here, and `p_gate` is what went to 0.

**Retries:** any retry switches from the prior basis to L1 once the model is warm (`forecast_arm`: attempt ≥ 2 uses
L1). One failed try is enough.

**The log line:** it says "the attempt runs on the ladder's choice", but `action=abandon` reads as if roko gave up. In
shadow mode it should say it is advice only.

### Second case: phase 4.3 run `3752b8ff`, T2#2 (2026-10-26, hermes-pilot)

It happened again in the next phase's run, with Undertow on roko main f50dc58fa and the same shadow model:

```
2026-10-26T14:37:02.913149Z  WARN roko_cli::graph_task_dispatch::self_model: self-model: T2 is unlikely to pass on any
rung; the attempt runs on the ladder's choice plan_id=p4-3-the-editor-s-finish task_id=T2
attempt_key=3752b8ff-83df-447e-bf79-b97db67b9bc7:p4-3-the-editor-s-finish:T2:2 action=abandon spec_score=Some(1.0)
best_p_vs=1.0727327479924159e-12
```

The run's predictions are copied to `tmp/hermes-roko-v7/pilot/evidence/p4-3-run/predictions-3752b8ff.jsonl`:

| Try | ts (Z) | trained_on_n | basis | mini p_vs (lcb) | strong p_vs (lcb) | decision | outcome |
|---|---|---|---|---|---|---|---|
| T2#1 | 14:33:51.675 | 67 | prior | 0.597 (0.495) | 0.723 (0.584) | start, mini | gate_failed on mid (45-turn cap) |
| **T2#2** | **14:37:02.913** | **68** | **logit** | **1.57e-15** (1.57e-15) | **1.07e-12** (1.07e-12) | **abandon** | gate_failed on mid |
| T2#3 | 14:40:28.467 | 69 | logit | 0.19 (0.0016) | 0.266 (0.0049) | retry | **passed on strong** ($0.32) |

- Here the forecast saturated without the isotonic map's exact 0 (1e-15 and 1e-12, not 0.0). That is cause 1 alone.
- One more training row later (n 69), the same task's forecasts swung to 0.19 and 0.27. This is the overshoot back
  and forth described above.
- In active mode the abandon would have stopped T2 two tries before the strong try that passed, leaving phase 4.3
  undelivered.

## Where

- `crates/roko-learn/src/self_model/logit.rs`: `OnlineLogit::update` and `forecast`.
- `crates/roko-learn/src/self_model/recal.rs`: `IsotonicMap::fit` and `apply`.
- `crates/roko-learn/src/self_model/model.rs`: `forecast_arm`, which picks the basis.
- `crates/roko-cli/src/graph_task_dispatch/self_model.rs`: the WARN line and the abandon action.

## Plan

1. **Bound L1's update in score space.** Scale each feature's step by `1/(1 + v·p(1 − p))`, or by `1/max(1, v)`: an
   assumed-density-filtering step, so one outcome moves the score by at most about one logit. Optionally scale hashed
   `bow:` features by 1/√(active count).
2. **Smooth the isotonic blocks.** Add a pseudo-count toward each block's mean forecast, e.g. (Σwy + p̄)/(Σw + 1), so
   no map returns exactly 0 or 1 from a few outcomes.
3. **Fall back on a degenerate forecast.** When L1's raw score is saturated (|z| > 20, or p outside [1e-6, 1 − 1e-6]),
   the candidate takes the prior basis and records `basis: prior` with why.
4. **Shadow wording.** In shadow mode the WARN line says it is advice only, e.g. "shadow self-model (advice only):
   would abandon T1 …; the attempt runs on the ladder's choice".
5. **Abandon needs evidence.** An abandon needs the prior's lcb to agree, not one model's 0.0 alone.

## Done when

A test replays the shape of this state, a warm model whose L1 saw dozens of hashed features with mixed outcomes. A
retry's forecast is then neither exactly 0 nor saturated, the isotonic map never returns 0 or 1 for a block of a few
outcomes, the shadow line says it is advice, and no abandon is proposed when the prior's lcb is above the floor.
