+++
id = "bug-a400d0"
kind = "bug"
title = "M3 learns each settled attempt under the provider slug it ran but forecasts its candidates under their [models.*] keys, so per-model learning never reaches a forecast when the two differ"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-cli/runtime_feedback", "roko-cli/graph_task_dispatch", "roko-learn/self_model"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "gap-b38ed2 worker, 2026-10-16: the calibration window missed every outcome of a key-not-slug run"
discovered_from = "gap-b38ed2"
anchors = ["crates/roko-cli/src/runtime_feedback/self_model.rs::unit_of", "crates/roko-cli/src/graph_task_dispatch/self_model.rs::arm_of", "crates/roko-learn/src/self_model/prior.rs::HierarchicalBeta", "crates/roko-learn/src/self_model/model.rs::SelfModel"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-b38ed2", "spec-7a71d6", "gap-e464e6", "gap-51d2ca"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn self_model_learns_the_arm_it_forecasts' crates/roko-cli/ && cargo test -p roko-cli self_model_learns_the_arm_it_forecasts"
+++

## Problem

The live self-model (M3) learns each settled attempt under one name for its model and forecasts it under
another, so when the two differ nothing it learns about a model reaches that model's forecasts.

- Dispatch forecasts the candidates under their `[models.*]` keys: `Candidates::of` builds each arm with
  `arm_of(config, &rung.model)` (or the pinned key), and `arm_of` keeps the key it is given
  (`crates/roko-cli/src/graph_task_dispatch/self_model.rs`).
- The outcome sink teaches the model a `Unit` whose arm names the model the provider bridge launched:
  `unit_of` takes `executed.model_dispatched`, which `executed_model` fills with `dispatch.target.model_slug`, the
  provider slug, before `executed.model_requested`, the key (`crates/roko-cli/src/runtime_feedback/self_model.rs`,
  `crates/roko-cli/src/graph_task_dispatch/attempt.rs::executed_model`).
- Everything M3 learns per model is keyed by that name: L0's strata `model`, `model|family` and
  `model|family|role` (`crates/roko-learn/src/self_model/prior.rs::HierarchicalBeta::keys`), L1's features and
  the price tier looked up by `price_tiers.get(&arm.model)`, the cost model's features and the (family, model)
  pass EMA (`crates/roko-learn/src/self_model/model.rs`).

Whenever a key is not its slug, a forecast of the key reads only strata the slug's outcomes never touch: the
model level of L0 falls back to the global stratum, and L1 and the cost model see an unknown model. This repo's
own `roko.toml` has `[models.glm-4-7] slug = "glm-4.7"`; the plan-runner test fixtures map `graph-model` to
`claude-sonnet-4-6` and the 6128 fixture `cheap-model` to `claude-haiku-4-5`.

Found while doing gap-b38ed2: the calibration window missed every outcome of such a run for the same reason
(`AttemptForecast::for_model(&unit.arm.model)` found no candidate). gap-b38ed2 fixed the window alone, by
looking the candidate up by `executed.model_requested` first (`SelfModelRuntime::settle`'s `requested`); the
learning itself still uses the slug.

## Why it matters

Goal `cybernetic`, epic spec-7a71d6 stage 3. M3's forecasts cannot separate the models of a ladder whose keys
are not slugs, so its resolution (BSS ≥ 0.05, AUROC ≥ 0.65) cannot reach the calibration gate's bounds however
long the window grows, policy (b)'s start rung and climbs rest on the global pass rate, and M1's `m3_prior`
reads the same flat forecasts. Related: gap-b38ed2 (the window), gap-e464e6 (L1's swings), gap-51d2ca (the
router after M3 calibrates).

## Where

- `crates/roko-cli/src/runtime_feedback/self_model.rs::unit_of`: the learned arm's model.
- `crates/roko-cli/src/graph_task_dispatch/self_model.rs::arm_of` and `Candidates::of`: the forecast arms.
- `crates/roko-learn/src/self_model/model.rs::SelfModel` (`observe_with`, `forecast_arm`, `price_tiers`) and
  `crates/roko-learn/src/self_model/prior.rs::HierarchicalBeta`.
- The offline fit names arms by slug too (`crates/roko-learn/src/self_model/ingest.rs::unit_of`), so it does not
  show the problem.

## Current state

Open. gap-b38ed2 matched the window's candidate by key; nothing else changed. `post_pass` already maps between
the two names (`ran_model`: the same name, or the `[models.*]` key of the slug).

## Plan

Name an arm's model one way on both paths. Options:

1. Forecast by slug: `arm_of` resolves a key to its `[models.*]` slug, which the price snapshot, the offline fit
   and S04's arm strings (`roko/cerebras/gpt-oss-120b@default#V0`) already use. The prediction rows then name
   slugs; `default`/`would_choose` must still map back to the ladder's rung (by index, as `Candidates` keeps it).
2. Learn by key: `unit_of` names the arm by `model_requested`. Smaller, but the live state and the offline fit
   then name arms differently, and price tiers keyed by slug still miss.

Option 1 looks right; check `post_pass`, `active_rung` and M1's `m3_prior` reader, which compare arm names.
Add the test `self_model_learns_the_arm_it_forecasts`: with two ladder rungs whose keys differ from their slugs,
one that always passes and one that always fails, after N_MIN settled attempts of each through the outcome sink
the passing rung's forecast is clearly above the failing one's.

## Done when

The `[[verify]]` command passes.

## Notes

The predictor version hashes only the class, the feature schema and the price snapshot, so a `state-v1.json`
saved before the change loads as the same version and keeps strata under the old names that no forecast reads.
Bump the feature schema (a new version starts fresh, and its calibration window with it) or accept the leftovers.
