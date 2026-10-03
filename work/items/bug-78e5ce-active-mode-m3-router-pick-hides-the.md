+++
id = "bug-78e5ce"
kind = "bug"
title = "Active-mode M3: router_pick hides the self-model's pick, streaming dispatch skips forecasting, early-climb eligibility may be stale"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-cli/graph-task-dispatch"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-9 follow-up reports 2026-10-03 (gap-d2d750)"
discovered_from = "gap-d2d750"
anchors = ["crates/roko-cli/src/graph_task_dispatch/ladder.rs::AttemptLadder", "crates/roko-cli/src/graph_task_dispatch/streaming.rs", "crates/roko-cli/src/graph_task_dispatch/self_model.rs::SelfModelRuntime"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn streaming_dispatch_forecasts_through_the_self_model' crates/roko-cli/ && cargo test -p roko-cli streaming_dispatch_forecasts_through_the_self_model"
+++

## Problem

Three gaps in the active-mode M3 self-model chain, surfaced by its own work (gap-d2d750, done):

1. **`AttemptLadder.router_pick` shows the self-model's pick, not the cascade router's, on L-M3 rows.**
   `crates/roko-cli/src/graph_task_dispatch/ladder.rs:196,199`: `router_pick: plan...and_then(|decision|
   decision.proposals.learned.clone())` — `router_pick` is sourced unconditionally from `decision.proposals.learned`.
   When the self-model is active and chooses the start rung (gap-d2d750's own feature), it writes *its own* pick
   into that same `proposals.learned` field, which `CascadeRouter`'s shadow pick normally occupies. So a field
   named for "the router's pick" silently means something different depending on whether M3 is active — anyone
   reading `router_pick` on an L-M3 row sees the self-model's choice under the router's name.
2. **The streaming dispatch path never calls the self-model's forecast hook.** The non-streaming path sets
   `dispatch_ctx.self_model_rung = self.forecast_attempt(spec, &task, &dispatch_ctx, &attempt)`
   (`crates/roko-cli/src/graph_task_dispatch.rs:1216`); the streaming path
   (`crates/roko-cli/src/graph_task_dispatch/streaming.rs:196`) just hardcodes `self_model_rung: None` with no
   call to `forecast_attempt` at all. Every task dispatched through the streaming path silently skips M3
   forecasting, regardless of whether the self-model is active.
3. **An early climb's re-check only genuinely re-evaluates the breaker, not full eligibility.**
   `SelfModelRuntime::post_failure_step` (`crates/roko-cli/src/graph_task_dispatch/self_model.rs:428`):
   `if self.settings.mode != SelfModelMode::Active || self.gate().breaker_tripped { return None; }` — this reads
   `self.settings.mode` and calls `self.gate().breaker_tripped` together, but `self.settings` appears to be fixed
   for the lifetime of the runtime instance (set once at construction/chain start), so the mode half of this
   check can't actually observe a mode change mid-chain — only `breaker_tripped` is a genuinely live re-check.
   If the self-model's mode can in fact change while a chain is in flight (a promotion/demotion applied
   mid-run), an early climb's eligibility re-check would miss it. (Whether `self.settings` is ever updated after
   construction needs confirming at implementation time — this is reported, not independently proven here.)

## Why it matters

Goal: cybernetic, M3 self-model (S04). (1) makes a telemetry field ambiguous exactly when it matters most — when
M3 is active and its pick is what anyone debugging a routing decision would want to see distinctly from the
cascade's. (2) means M3's whole forecasting apparatus has a silent blind spot on an entire dispatch path. (3) is
a potential staleness gap in a safety-relevant re-check, if mode really can change mid-chain.

## Where

- `crates/roko-cli/src/graph_task_dispatch/ladder.rs::AttemptLadder` construction (1).
- `crates/roko-cli/src/graph_task_dispatch.rs` (forecast call) vs.
  `crates/roko-cli/src/graph_task_dispatch/streaming.rs` (no call) (2).
- `crates/roko-cli/src/graph_task_dispatch/self_model.rs::SelfModelRuntime::post_failure_step`,
  `::settings`/`SelfModelMode` (3).

## Current state

(1) and (2) confirmed directly. (3)'s core mechanism (the dual check) is confirmed; whether `self.settings.mode`
can change post-construction is not yet confirmed either way.

## Plan

1. Give `AttemptLadder` a separate field (or a tag on `router_pick`) distinguishing "the cascade's shadow pick"
   from "the self-model's active-mode pick," so the two are never conflated under one name.
2. Add the missing `forecast_attempt` call to the streaming dispatch path, mirroring the non-streaming one.
3. Confirm whether `SelfModelRuntime.settings` can change after construction; if it can, make
   `post_failure_step`'s mode check read the live value, not a cached one.

## Done when

- `router_pick` (or its replacement) never shows the self-model's pick under the router's name.
- The streaming dispatch path forecasts through M3 the same way the non-streaming path does.
- Early-climb eligibility re-checks the self-model's current mode live, if it can change mid-chain; documented
  as intentionally fixed-at-start otherwise.
- The `[[verify]]` command passes.

## Notes

- Discovered during the work that closed gap-d2d750.
