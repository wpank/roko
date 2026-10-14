+++
id = "bug-3e2d76"
kind = "bug"
title = "active_self_model_picks_start_rung_and_logs_propensity fails on days whose exploration draw lands on the greedy model"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/dispatch"]
created = 2026-10-14
updated = 2026-10-14
last_verified = 2026-10-14
source = "roko-loops worker, full roko-cli lib run on 2026-10-14"
anchors = ["crates/roko-cli/src/dispatch/model_routing.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli active_self_model_picks_start_rung_and_logs_propensity"
+++

## Problem

`dispatch::model_routing::tests::active_self_model_picks_start_rung_and_logs_propensity` fails on 2026-10-14, 40 runs
out of 40, on a tree whose changes do not touch routing (the roko-loops items, stacked on c62108cf5):

```
assertion `left == right` failed
  left: Some(Explore)
 right: Some(SelfModel)
```

The ε-exploration draw is keyed by the day (`chrono::Utc::now().format("%Y-%m-%d")`, model_routing.rs, the explore
draws at ~819 and ~1329). With ε = 0.25 the test's chain explores on some days, and an exploring draw can land on the
greedy model, claude-sonnet-4-6 (its p is 0.75 + 0.0625). The test infers the expected source from the chosen model
alone (`if choice.model.slug == "claude-sonnet-4-6" { SelfModel } else { Explore }`), so on such a day it expects
`SelfModel` for a row the router correctly logs as `Explore`.

## Why it matters

`cargo test --workspace`, a mandatory pre-commit check, fails on those days whatever the change under test, and the
test passes or fails by date rather than by code.

## Where

`crates/roko-cli/src/dispatch/model_routing.rs`, the test's ε section (the `exploring` router, about lines 2925-2942).

## Plan

Make the test independent of the date: pin the draw's epoch (an injectable clock or epoch on the router), or assert
the source the draw implies (explored or not, from the row's own draw fields) instead of inferring it from the model.

## Done when

`[[verify]]` passes on any date.
