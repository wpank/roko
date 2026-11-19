+++
id = "gap-b0fd8f"
kind = "gap"
title = "The self-model forecasts only a rung's own model, so a try a rung's draw ran on an alternate is learned but never forecast"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch", "roko-learn/self_model"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
source = "gap-51d2ca (2026-10-21)"
discovered_from = "gap-51d2ca"
anchors = ["crates/roko-cli/src/graph_task_dispatch/self_model.rs::Candidates::of", "crates/roko-cli/src/graph_task_dispatch/self_model.rs::SelfModelRuntime::settle", "crates/roko-cli/src/dispatch/model_routing.rs::RoutingLadder::rung_choices"]
lane = "rust-hot"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = ["gap-51d2ca", "gap-96547f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn self_model_forecasts_rung_alternates' crates/roko-cli/ && cargo test -p roko-cli self_model_forecasts_rung_alternates"
+++

## Problem

The self-model (M3) forecasts one model per ladder rung, the rung's own: `Candidates::of`
(`crates/roko-cli/src/graph_task_dispatch/self_model.rs`) builds each candidate arm from `rung.model`. Since
gap-51d2ca a rung may name `alternates` and draw among them at ε (`RoutingLadder::rung_choices`,
`crates/roko-cli/src/dispatch/model_routing.rs`). The workspace's `roko.toml` now does so: `mid` runs `kimi-k2-5` on
ε/2 of its tries.

What happens to such a try:
- The outcome sink teaches the model the unit under the alternate's arm (`SelfModelRuntime::settle` → `observe`).
- `settle` finds no forecast for the model that ran (`AttemptForecast::for_model`), so the calibration gate's
  window and policy (a)'s margin skip the attempt.
- Its prediction row names the rung's own model as `default`, though another model ran.
- In active mode M3 can never weigh an alternate: its start rule and climbs choose rungs, each scored by its own
  model's forecast.

## Why it matters

Goal `cybernetic` (S04). The window's sample loses a random ε/k share of a drawing rung's tries. That share is
unbiased, but it is the only data where the rung's models are compared on the same tasks, which is what S04's
per-arm forecasts would need to tell a rung's models apart. Joining prediction rows with verdicts
(`roko learn self-model fit`, gap-96547f) sees a `default` that did not run.

## Where

- `crates/roko-cli/src/graph_task_dispatch/self_model.rs::Candidates::of`, `SelfModelRuntime::settle`,
  `AttemptForecast::for_model`.
- `crates/roko-cli/src/dispatch/model_routing.rs::RoutingLadder::rung_choices`: a rung's models.

## Current state

Open. Found while doing gap-51d2ca (2026-10-21). The behaviour is safe as it stands: shadow mode never routes, and an
active self-model's route runs each rung's own model with no draw (`ModelRouter::decide_self_model`).

## Plan

A design choice for S04:

1. Forecast every model of a rung: each runnable alternate becomes a candidate on its rung's index, so the window
   scores the model that ran and the prediction row can name the draw's pick.
2. Keep M3 on a rung's own model, and record in the prediction row that a rung's draw ran another (so readers do not
   take `default` for the executed model).

Test `self_model_forecasts_rung_alternates`: with a two-model rung, an attempt that ran the alternate settles into
the calibration window under its own forecast (option 1), or its prediction row says the draw ran another model
(option 2).

## Done when

The `[[verify]]` command passes.
