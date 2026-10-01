+++
id = "bug-982600"
kind = "bug"
title = "A ModelCallService cache hit counts as a fresh router trial and adds cost in the gateway stats"
status = "open"
triage = "unverified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-learn", "roko-serve"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-c1f6b8"
anchors = ["crates/roko-learn/src/feedback_service.rs", "crates/roko-serve/src/routes/gateway.rs"]
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
