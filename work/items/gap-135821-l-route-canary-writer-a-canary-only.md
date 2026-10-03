+++
id = "gap-135821"
kind = "gap"
title = "L-route canary writer: a canary-only preference in CascadeRouter, task 5127's remainder"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/loop-audit"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "tmp/backlog/2026-10-02-complete-and-wire 5127 (partial in wave 8, PK43)"
discovered_from = "gap-c1d920"
anchors = ["crates/roko-learn/src/cascade_router.rs", "crates/roko-learn/src/loop_audit"]
lane = "rust-cold"
links = { depends_on = ["gap-c1d920"], blocks = [], related = ["gap-c1d920"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn route_canary_reaches_the_router_and_is_removed' crates/ && cargo test -p roko-cli route_canary_reaches_the_router_and_is_removed"
+++

## Problem

Task 5127 of PK43 (gap-c1d920, gated in wave 8) builds production canary writers for L-know, L-play and L-route.
PK43's worker built L-know's and L-play's; L-route's can't be built as the spec says. S03 §4.7 asks for a router
preference that sends the synthetic `canary` category to a cheap model, but the cascade router keys everything it
learns by closed enums (`TaskCategory`, `AgentRole`), so no canary-scoped preference exists. The only preference a
canary could set and remove exactly is the role-to-model table, which applies to every task of a role and is read
only while the router is cold.

## Why it matters

Without an L-route canary the loop auditor can't localise a break in the routing loop (S03's canary probe, 5121).

## Where

`crates/roko-learn/src/cascade_router.rs`, the canary driver (`roko_learn::loop_audit::canary`, 5121), and
`crates/roko-cli/src/loop_canary.rs` (PK43's writers).

## Current state

L-know and L-play canary writers are merged (gate 8c); L-route has none.

## Plan

1. The coordinator chose (2026-10-03) a canary-only key inside `CascadeRouter` (a preference keyed by the canary
   category or nonce, consulted only for canary tasks and removed exactly), rather than a new
   `TaskCategory::Canary` variant in roko-core, which would touch every match on the enum across crates. Will may
   overrule.
2. Implement the L-route writer through it, and extend PK43's `canary_writers_reach_real_readers` (or add a test)
   so the L-route canary reaches the router's real reader and is removed afterwards.

## Done when

- [ ] The `[[verify]]` command passes.

## Notes

- Full spec: `tmp/backlog/2026-10-02-complete-and-wire/5127-production-canary-writers-know-play-route.md`.
