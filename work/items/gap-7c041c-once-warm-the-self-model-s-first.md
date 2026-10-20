+++
id = "gap-7c041c"
kind = "gap"
title = "Once warm, the self-model's first L1 forecasts are saturated and follow the last outcome, not the lesson: worse than the L0 forecast they replace, and the calibration gate stays shut for 24 tasks"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/self_model"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "round-v6-learning-research#5.2"
discovered_from = "agent:learning-research (round v6, 2026-10-17, lessons-on start-rung demo on roko-92cc56-3f36a6-merged at 5ca1cb3ac)"
anchors = ["crates/roko-learn/src/self_model/logit.rs::OnlineLogit", "crates/roko-learn/src/self_model/model.rs::N_MIN", "crates/roko-learn/src/self_model/features.rs::feature_vector", "crates/roko-learn/src/self_model/recal.rs::Recalibrator"]
lane = "rust-cold"
parent = "spec-7a71d6"
links = { depends_on = ["gap-e464e6"], blocks = [], related = ["gap-3f36a6", "bug-92cc56", "gap-dd89ff"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn first_warm_forecasts_beat_l0_on_a_deciding_feature' crates/roko-learn/ && cargo test -p roko-learn first_warm_forecasts_beat_l0_on_a_deciding_feature"
+++

## Problem

Once the self-model is warm (`N_MIN = 30` outcomes, `crates/roko-learn/src/self_model/model.rs`), L1 replaces L0
outright, and its first forecasts are saturated. They follow the last outcome, not the features that decide it.

This shows in the $0 start-rung demo's lessons-on series (round v6). Settings:
- binary `roko-92cc56-3f36a6-merged` (5ca1cb3ac), which has gap-dd89ff's content features;
- scripted models: cheap passes its first try exactly when the prompt carries the stored lesson, while mid and strong
  always pass;
- `[self_model] mode = "active"`, `n_min = 12`;
- the lesson withheld on 23 of 44 chains.

What the forecasts did:
- **Tasks 17–32 (`trained_on` 30–65).** The first 16 tasks on the logit.
  - **Saturated:** cheap's first-try forecast was saturated on 14 of 16. At task 17, `p_gate` was 0.9999999982:
    `p_vs` 0.950 is `p_gate` × (1 − the 0.05 false-green prior), and `p_vs_lcb` was 0.936. At task 19, `p_gate` was
    0.00057.
  - **Follows the last outcome:** it fell on the same side of 0.5 as the previous cheap first try on 16 of 16.
  - **Ignores the lesson:** it fell on the lesson's side on only 8 of 16:
    - 0.94–0.95 on chains without the lesson (tasks 18, 20, 25 and 29);
    - 0.001–0.22 on chains with it (tasks 19, 23, 28 and 32).
- **From task 33 the content features took over:** 13 of 13 on the right side, 0.57–0.95 with the lesson and
  0.000–0.47 without.
- **The gate:** its window is 12. ECE stayed above 0.08 until task 40 (0.348 at task 16, 0.124 at task 40), so the
  gate first held after task 41. The self-model then started 4 of 4 tasks on the right rung.

The mechanism is gap-e464e6's update rule.
- **The update.** `OnlineLogit::update` grows each feature's curvature by `p(1 − p)x²`. That is close to 0 once a
  forecast is near 0 or 1, so the next step stays large.
- **The always-on block.** In this vector about 35 features are on for every task of the family:
  - `bias`;
  - about 20 shared bag-of-words buckets;
  - family, tier, role, harness, effort and depth;
  - the playbook arm;
  - the eight `spec_missing:*` slots.
- **What one outcome does.** Each always-on feature takes the full step, so the score moves many log-odds and lands on
  the side of the last outcome. The lesson moves only two features: the `knowledge_arm` one-hot and
  `knowledge_carried`.
- **Nothing checks the handover.** L1 takes over at `N_MIN` without any check that it beats L0. The isotonic map in
  `recal.rs`, by contrast, is used only while it beats the raw forecasts.

A Python replica reproduces this:
- **What it models:** `logit.rs`'s update, a Beta(1, 1) L0 per model, the demo's feature groups and chain shape, the
  first 16 warm tasks over 5 seeds. It leaves the recalibrator out.
- **Where:** `tmp/hermes-roko-v6/learning-research/replica/l1_handover.py`.

| Always-on features | Update rule | Right side of 0.5 | Follows the last first try | Saturated | Brier (L0's: 0.278) |
|---|---|---|---|---|---|
| 30 | today (`p(1 − p)x²`) | 52/80 | 77/80 | 73/80 | 0.333 |
| 30 | gap-e464e6's bound (`x²/4`) | 74/80 | 55/80 | 20/80 | 0.066 |
| 45 | today | 49/80 | 80/80 | 75/80 | 0.373 |
| 45 | gap-e464e6's bound | 69/80 | 60/80 | 35/80 | 0.112 |
| 15 | today | 74/80 | 55/80 | 66/80 | 0.051 |

- **Today's rule** is worse than the L0 forecast it replaces, and it gets worse as the family's shared task text
  grows.
- **The bound alone** beats L0 at every size tried.
- **A handover that also requires L1 to beat L0's recent Brier** did slightly worse than the bound alone (0.139 at
  30 features).

## Why it matters

Goal: cybernetic (S04, M3).
- **The content features can't steer.** For about 35 outcomes after warm-up, gap-dd89ff's lesson features cannot move
  the forecast. A self-model that should keep lesson-carrying tasks on cheap instead flips with the last outcome.
- **The gate stays shut.** Forecasts that are confidently wrong half the time fail ECE, so active mode cannot act for
  many tasks after warm-up. Here that was 24 tasks, and one scripted family with one clean cause of failure is the easy
  case.
- **The forecasts get worse at warm-up.** Anyone reading `predictions.jsonl` or `route_decided.forecast` in a freshly
  warm workspace sees confident flips. The lower bound doesn't protect either: `p_vs_lcb` was 0.936 on a forecast
  that was wrong.
- **First tries.** gap-3f36a6 may be fixed by forecasting every first try from L0's per-arm rate; that is being tried
  on roko-recovery's branch. If so, first tries stop using L1 and the start choice cannot see the lesson, while this
  saturation still reaches retries and the gate's window. Once this item is fixed, first tries could go back to L1 to
  get lesson-aware starts.

## Where

- `crates/roko-learn/src/self_model/logit.rs::OnlineLogit` (`update`: the curvature; gap-e464e6).
- `crates/roko-learn/src/self_model/model.rs::N_MIN` and `cold_start`: L1 replaces L0 outright.
- `crates/roko-learn/src/self_model/features.rs::feature_vector`: the always-on block (bag-of-words, `spec_missing:*`).
- `crates/roko-learn/src/self_model/recal.rs::Recalibrator`: its rule that a map must beat the raw forecasts, the
  pattern for a handover check.

## Current state

- Open; nothing addresses it.
- gap-e464e6's decided fix (2026-10-15: grow `h` by `w·x²/4`, Böhning's bound) is not on main: `logit.rs` still adds
  `w·p(1 − p)·x²`.
- gap-e464e6's test covers a fixed mixed pass rate with no informative feature. It does not cover a feature that
  decides the outcome.

## Plan

1. Land gap-e464e6's bound (this item depends on it).
2. Add `first_warm_forecasts_beat_l0_on_a_deciding_feature` in roko-learn, built like the demo:
   - It goes through `SelfModel::observe_with` with `TaskFeatures` whose content group carries the lesson on a random
     half of the chains.
   - With the lesson, cheap passes its first try. Without it, cheap fails twice and mid passes.
   - The task text is the demo's: `make_rung_project.py` in `tmp/hermes-roko-v6/learning-research/scripts/`.
   - Run it over 5 seeds. Score L1's forecast for cheap's first try (`GateForecaster` with L0's offset) on the first
     16 tasks after `N_MIN`, so the test holds whichever model forecasts first tries.
   - It must score a lower Brier than L0's per-arm forecast on the same outcomes, and land on the right side of 0.5 for
     at least 80% of them.
3. If the bound alone does not pass, hand first tries from L0 to L1 only while L1's Brier on recent outcomes beats
   L0's, as `recal.rs` does for its map.
4. Re-run the demo's half-holdout series as the end-to-end check: `scripts/rung_rerun.sh <binary> <tag>`, series
   `fix-lessons-h50`, $0. At 5ca1cb3ac the gate first held after task 41.

## Done when

- `first_warm_forecasts_beat_l0_on_a_deciding_feature` passes for 5 seeds.
- The `[[verify]]` command passes.

## Notes

- Evidence:
  - `tmp/hermes-roko-v6/findings/learning-research.md` §5.2;
  - `tmp/hermes-roko-v6/learning-research/evidence/start-rung/merged/`: `series-fix-lessons-h50/`, the last table in
    `table.md`, and `summary-lessons-h50.json`.
- Related:
  - gap-3f36a6: the attempt-order confound of the first-try forecast;
  - bug-92cc56: the gate's routed-pass-rate bound;
  - gap-dd89ff: the content features.
