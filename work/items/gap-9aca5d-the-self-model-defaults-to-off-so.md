+++
id = "gap-9aca5d"
kind = "gap"
title = "The self-model defaults to off, so it never forecasts: make shadow the default (decision 2 of 2026-10-15 supersedes 6102's off)"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-core/config", "roko-cli/graph_task_dispatch"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#decision-2"
anchors = ["crates/roko-core/src/config/self_model.rs::SelfModelMode", "crates/roko-core/src/config/self_model.rs::SelfModelConfig", "crates/roko-core/src/config/self_model.rs::self_model_config_defaults_off", "crates/roko-cli/src/graph_task_dispatch/self_model.rs::SelfModelRuntime"]
lane = "rust-cold"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = ["gap-e464e6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn self_model_config_defaults_shadow' crates/roko-core/ && cargo test -p roko-core self_model_config_defaults_shadow"

[[verify]]
command = "grep -rqw 'fn default_plan_run_forecasts_in_shadow' crates/roko-cli/ && cargo test -p roko-cli default_plan_run_forecasts_in_shadow"
+++

## Problem

The self-model (M3) is `off` by default: `SelfModelMode` has `#[default] Off` and `SelfModelConfig::default()` sets
`mode: SelfModelMode::Off` (`crates/roko-core/src/config/self_model.rs:20`, `:67`; the module doc at `:4` cites
decision 6102 question 5). Neither `roko.toml` nor `~/.roko/config.toml` has a `[self_model]` section, so
`SelfModelRuntime::load` (`crates/roko-cli/src/graph_task_dispatch/self_model.rs:206-209`) returns `None` on every run:
no forecast, no `predictions.jsonl`, no `learn/self-model/state-v1.json` exists anywhere (A07 §5).

## Why it matters

Goal `cybernetic` (S04). Will's decision 2 (2026-10-15) replaces decision 6102's "off" with "shadow": the forecaster
runs in shadow by default, which is what the papers say (D5 8-39 "the forecaster runs in shadow first"; D7 A.8, A-20).
Shadow forecasts every routed attempt and logs its would-be rung; it never routes (`routes()` needs `active`). Without
shadow data, M3 can never become eligible, M1's `m3_prior` (default true) has nothing to read, and gap-b38ed2/gap-96547f have
nothing to measure. Stage 2 of the epic.

## Where

- `crates/roko-core/src/config/self_model.rs::SelfModelMode` and `SelfModelConfig` (`Default`), the module doc, and the
  test `self_model_config_defaults_off`.
- `crates/roko-cli/src/graph_task_dispatch/self_model.rs::SelfModelRuntime` (`load`: needs a price snapshot through
  `PriceSnapshot::shared`; `save` at run end, `plan_runner.rs` "Persist the self-model").

## Current state

Shadow and active hooks exist and are tested (6122, 6126, 6128-6130 code done; A07). gap-e464e6 (open): once warm, L1
forecasts swing between 0.03 and 0.97; harmless in shadow, but it will show in calibration (gap-b38ed2).

## Plan

1. Make `Shadow` the `#[default]` variant and the `Default` mode; update the module doc to cite decision 2 of
   2026-10-15, superseding 6102 (5).
2. Rename the test to `self_model_config_defaults_shadow` and assert `mode = shadow` with the other defaults unchanged.
3. Sweep tests that assume no self-model files on a default run (for example ones that list `.roko/learn`), and frozen
   runs (`[learning] frozen` must still save nothing).
4. Add `default_plan_run_forecasts_in_shadow`: a fake-provider plan run with no `[self_model]` section writes one
   prediction row per routed attempt and the state file, and routes exactly as with `mode = off`.

## Done when

Both `[[verify]]` commands pass.

## Notes

- No model call: the forecaster is local. When no price snapshot loads, `load` still returns `None`; that is fine.
- Record on backlog decision 6102 (its `DECISIONS.md` row) that question 5 is superseded by decision 2 of 2026-10-15.
- `active` stays a person's switch (6102 (2ii)).
