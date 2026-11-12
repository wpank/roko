+++
id = "gap-03d8bc"
kind = "gap"
title = "A run with learning frozen still draws ε-exploration on the cascade router's routes, while a rung's draw among its alternates stops"
status = "open"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-cli/dispatch", "roko-core/config"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
source = "gap-51d2ca (2026-10-21)"
discovered_from = "gap-51d2ca"
anchors = ["crates/roko-cli/src/dispatch/factory.rs::SharedAgentFactory::new", "crates/roko-core/src/config/routing.rs::RoutingConfig", "crates/roko-core/src/config/learning.rs::LearningConfig"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-51d2ca"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn frozen_run_draws_no_route_exploration' crates/roko-cli/ && cargo test -p roko-cli frozen_run_draws_no_route_exploration"
+++

## Problem

A run with `[learning] frozen` still draws ε-exploration on the routes the cascade router decides.
`SharedAgentFactory::new` (`crates/roko-cli/src/dispatch/factory.rs`) gives the router
`with_explore_epsilon(config.routing.effective_explore_epsilon())` whatever `learning.frozen` says; only
`[experiments] maximize` sets ε to 0. A frozen run registers no routing sink
(`crates/roko-cli/src/graph_execution/plan_runner.rs::build_graph_feedback_facade` keeps only the per-run episode
sink), so an explored route teaches the router nothing and only adds variance: at the default ε = 0.05 about one
ladder-free route in twenty runs a model other than the router's argmax.

gap-51d2ca made a ladder rung's draw among its `alternates` follow the flag (`with_rung_exploration(!frozen)`), so the
two exploration draws now disagree in a frozen run: rungs never draw, the router's routes still do.

## Why it matters

Goal `learning`. A frozen run is the fixed arm of an ablation or a bench comparison (decision 2218): it reads learned
state and writes none, and "a frozen run assigns no prompt experiments" (`crates/roko-core/src/config/learning.rs`,
`frozen` doc). Exploration that nothing learns from is cost and noise in exactly the runs meant to be reproducible.
Against that, the explored rows still carry logged propensities that off-policy evaluation can read
(`crates/roko-learn/src/self_model/ope.rs::logged_decisions`).

## Where

- `crates/roko-cli/src/dispatch/factory.rs::SharedAgentFactory::new`: the ε the router gets.
- `crates/roko-core/src/config/routing.rs::RoutingConfig::explore_epsilon` and
  `crates/roko-core/src/config/learning.rs::LearningConfig::frozen`: the documented contract.

## Current state

Open. Found while doing gap-51d2ca (2026-10-21); nothing has changed the ladder-free ε since 4114.

## Plan

A design choice:

1. ε = 0 in a frozen run, as in maximize mode, documented on both fields. Smallest, and consistent with rung draws.
2. Keep exploring and document why: the logged propensities are worth the variance.

Test `frozen_run_draws_no_route_exploration`: a factory built from a frozen config routes an unpinned, ladder-free
attempt to the router's argmax over many attempt keys, and its rows log propensity 1 (option 1); or the documented
behaviour for option 2.

## Done when

The `[[verify]]` command passes.
