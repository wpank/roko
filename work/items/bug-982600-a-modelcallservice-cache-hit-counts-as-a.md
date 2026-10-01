+++
id = "bug-982600"
kind = "bug"
title = "A ModelCallService cache hit counts as a fresh router trial and adds cost in the gateway stats"
status = "open"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-learn", "roko-serve"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "a788dfd8d"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-c1f6b8"
anchors = ["crates/roko-learn/src/feedback_service.rs", "crates/roko-serve/src/routes/gateway.rs", "crates/roko-agent/src/gateway_events.rs::GatewayEvent::billed_cost_usd"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-c1f6b8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-learn --lib cache_hits_are_not_router_trials"
+++

## Problem

FeedbackService observes every ModelCall event, so a cache hit counts as a fresh router trial. Serve's gateway stats route (`stats_by_model`, `routes/gateway.rs:412`) also sums cache hits' cost. The `cache_hit` field bug-c1f6b8 added lets both skip them.

## Plan

Skip cache hits in router learning and in the gateway's cost sums. Add a test named `cache_hits_are_not_router_trials`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-01 by wk-learn2, working on bug-c1f6b8, during the evening close-out round.
- 2026-10-02 (wk-settle): PARTIAL on work/bug-f9ae3e; cargo verification deferred to the batch check.
  - Gateway cost: the gateway log's `GatewayEvent` already records `cache_hit`, and a cache hit's event repeats the
    cached call's usage. `GatewayEvent::billed_cost_usd` is 0 for a cache hit, and the projection's per-model,
    per-provider, per-caller and total cost sums use it, so serve's `/api/gateway/stats` no longer adds cache hits'
    cost. Test: `cache_hits_add_no_gateway_cost`; `stats_by_model_aggregates_correctly` now expects the cache hit's
    cost left out.
  - Left: router learning. `FeedbackEvent::ModelCall` has no `cache_hit` at `c7560e213`; bug-c1f6b8 adds it in gate
    6d. Once that is on this branch, `FeedbackService::record` skips `observe_model_call` for a cache hit, with the
    test `cache_hits_are_not_router_trials` (this item's verify).
- 2026-10-02 (wk-settle): implemented on work/bug-f9ae3e after merging gate 6d (`a788dfd8d`); cargo verification
  deferred to the batch check. `FeedbackService::record` no longer observes a cache hit on the cascade router (cost
  rows already skip one, bug-c1f6b8). Test: `cache_hits_are_not_router_trials`.
