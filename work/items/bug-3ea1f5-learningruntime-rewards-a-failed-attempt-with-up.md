+++
id = "bug-3ea1f5"
kind = "bug"
title = "LearningRuntime rewards a failed attempt with up to 0.5 through cost and latency, unlike every other router path"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/runtime-feedback", "roko-learn/cascade-router"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:47, wk-router's report on bug-8da8ba, branch work/bug-8da8ba)"
anchors = ["crates/roko-learn/src/runtime_feedback/mod.rs::update_cascade_router", "crates/roko-learn/src/runtime_feedback/routing.rs::compute_reward_with_latency", "crates/roko-learn/src/model_router.rs::compute_routing_reward_with_weights"]
lane = "rust-cold"
parent = "spec-6ac537"
links = { depends_on = ["bug-8da8ba"], blocks = [], related = ["bug-f68404", "bug-dfb28f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_failed_episode_earns_zero_router_reward' crates/roko-learn/src/ && cargo test -p roko-learn --lib a_failed_episode_earns_zero_router_reward"
+++

## Problem

`LearningRuntime::update_cascade_router` (Path A, `runtime_feedback/mod.rs`) computes its reward with `compute_reward_with_latency(episode.success, cost, wall_ms, ..)`. For a failure the pass term is 0, but the cost and latency terms still count. With the default weights (quality 0.5, cost 0.3, latency 0.2, from `roko-core/src/config/routing.rs`) the reward is `0.3 * (1 - cost/$5) + 0.2 * (1 - latency/120 s)`: up to 0.5 for a cheap, fast failure. `record_observation` passes that reward to LinUCB unchanged (`observe_internal`).

After bug-8da8ba, every other path gives a failure a reward of 0: both Graph routing sinks (`observe_router_outcome`), the gateway, Path B (`FeedbackService`) and ACP. `observe_multi_objective_outcome` documents the rule: the cost and latency of a failed attempt bought nothing.

## Why it matters

The same failure teaches the router different things depending on which entry point ran it. Under Path A, a model that fails quickly and cheaply scores up to half of a success, so the bandit can keep choosing it. Routing is one of the cybernetic loops (epic spec-6ac537).

## Where

- `crates/roko-learn/src/runtime_feedback/mod.rs::update_cascade_router` (:1620 at BASE): the reward, the `record_observation` call, and the `WalEntry::CascadeObservation` it journals.
- `crates/roko-learn/src/runtime_feedback/routing.rs::compute_reward_with_latency` and `crates/roko-learn/src/model_router.rs::compute_routing_reward_with_weights`: the scalar reward.
- Path A's callers: roko-cli `agent_exec.rs`, `bench.rs` and `commands/util.rs` open a `LearningRuntime`.

## Current state

Checked at BASE and on `work/bug-8da8ba` (`26947cd62`): bug-8da8ba leaves Path A unchanged. The override path has the same problem in another form: a failed override is recorded as a success with quality 0 and zero cost and latency, so it earns 0.5. bug-f68404 covers that one.

## Plan

1. In `update_cascade_router`, use a reward of 0 when `episode.success` is false, and keep the latency-aware reward for successes. Journal the reward that was applied.
2. Put the failure rule in one helper in `cascade_router.rs`, shared by Path A, the sinks and the gateway, so the paths can't drift apart again.
3. Add `a_failed_episode_earns_zero_router_reward`: a failed episode with small cost and latency leaves the arm's LinUCB `b` vector at 0, as `routing_sink_updates_linucb_on_failure` checks for the Graph sink.

## Done when

- [ ] A failed episode on Path A updates the router with reward 0, like every other path.
- [ ] The `[[verify]]` command passes.

## Notes

- Depends on bug-8da8ba, which sets the reward-0 rule on the other paths (Rust batch 3).
- WAL entries written before the fix replay with their old reward. Decide whether replay should also clamp failure rewards to 0.
- Implemented on `work/bug-f68404` at `7e930d2cb`. Checked in the worker's own target dir: `cargo test -p roko-learn --lib` (1184 passed, including `a_failed_episode_earns_zero_router_reward` and `a_replayed_failure_earns_zero_reward`), fmt, and clippy `-D warnings`. The batch check re-verifies after merge.
- The rule is `cascade_router::outcome_reward`, applied in `observe_internal`, which Path A, the gateway, Path B, overrides and WAL replay all go through; the Graph sinks' multi-objective path already gave failures 0.
- Replay decision: replay clamps too. A WAL entry journaled before the fix replays a failure with reward 0 (`a_replayed_failure_earns_zero_reward`).
