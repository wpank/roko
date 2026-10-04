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
- 2026-10-04 (wave-10 follow-up, PK66/gap-414e56): this item's sub-finding (2) (the streaming path skips the
  self-model's forecast hook) also means it skips the whole `post_pass` step
  (`crates/roko-cli/src/graph_task_dispatch/self_model.rs::post_pass`/`post_pass_action`), not just forecasting —
  so a streaming-dispatched task's post-pass never writes a `spec.refine_requested` event
  (`SPEC_REFINE_EVENT`) either, even when the self-model would otherwise judge the task's spec needs refining.
  When fixing (2), confirm the fix restores `post_pass` and its `spec.refine_requested` event, not just the
  forecast call. Related: gap-2b0575 (S07's plan-load gate not reading that event at all yet, a separate,
  consumer-side gap even once it's produced).
- 2026-10-04 (wave-17b follow-up, work/backlog-batch-17b not yet merged): on that branch, facet
  (2) (streaming forecast) is done and facet (3) (early-climb eligibility) is resolved as
  intentionally fixed-at-start. Facet (1) (`router_pick`) is confirmed still not done, and
  implementing (2) surfaced a new residual: the streaming `DispatchContext` still passes
  `attempt: 0`, so the self-model's `has_prior_failure` is always false there. Both are filed
  separately — `bug-7dff88` (facet 1) and `bug-b087ea` (the `attempt: 0` residual) — since this
  item's single named `[[verify]]` command covers only the original facet 2 and could close
  before either lands.

## Progress

- (2) streaming forecast: implemented at 2155352e3 on `work/bug-78e5ce`; cargo verification deferred to the batch
  gate. `dispatch_streaming` calls `forecast_attempt` where the batch path does, which also restores 6133's
  refine/abandon reports, 6130's active start rung and the cached forecast that 6132's post-pass step and 6129's
  sink read. Test: `streaming_dispatch_forecasts_through_the_self_model` (`graph_task_dispatch/streaming.rs`).
  The streaming `DispatchContext` still passes `attempt: 0` (the batch path passes the attempt number), so a
  streaming retry's features say first attempt.
- (3) eligibility: documented as fixed at the run's start (`SelfModelRuntime::settings`, `post_failure_step`).
  The plan runner loads `[self_model]` once, nothing writes it, and the breaker is the live re-check.
- (1) router_pick: not done. Proposed fix: in `ladder.rs::record_attempt_ladder`, take `router_pick` from the
  route decision only when its `audit.loop_id` is not L-M3 (`SELF_MODEL_LOOP` in `dispatch/model_routing.rs`,
  which would need to become `pub(crate)`), since L-M3 rows carry the self-model's pick (a^L) as
  `proposals.learned`. Its pick stays in the prediction row and the route decision row.
