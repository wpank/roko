+++
id = "bug-35a738"
kind = "bug"
title = "Three M1 controller moves (B2, B6, TierFloor-down) are inert under current defaults and sequencing"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-cli/runner", "roko-cli/graph-task-dispatch", "roko-learn/homeostasis"]
created = 2026-10-03
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "d16bc9969"
source = "wave-9 follow-up reports 2026-10-03 (PK63 gap-eb39c1)"
discovered_from = "gap-eb39c1"
anchors = ["crates/roko-cli/src/runner/promise_tracker.rs::PromiseTracker", "crates/roko-cli/src/graph_task_dispatch/retry_budget.rs", "crates/roko-learn/src/homeostasis/catalog.rs::KnobKind"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn b2_retry_delta_has_room_to_move_on_a_ladder_routed_task' crates/roko-cli/ && cargo test -p roko-cli b2_retry_delta_has_room_to_move_on_a_ladder_routed_task"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T13:06:39Z"
commit = "d16bc9969"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-04T10:35:25Z"
forced = false
evidence = "Gate 17b (merged d16bc9969): verify b2_retry_delta_has_room_to_move_on_a_ladder_routed_task passes, plus search_never_turns_an_inert_knob; homeostat_disturbances passes. SafetyBox rule 8 (InertMove) refuses B6 and TierFloor-down below theta0 as search moves; B2 stays admissible and moves once S5 widens the box. Room for B2 at the defaults is Will's call (q-85792e)."
+++

## Problem

Three of M1's controller moves are inert under current defaults/sequencing, surfaced by PK63 (gap-eb39c1, done):

1. **B6 (promise moves) never fires in practice.** `PromiseTracker::record_and_check`
   (`crates/roko-cli/src/runner/promise_tracker.rs`, `consecutive_threshold = 2`) only triggers
   `PromiseDecision::Terminate` after two consecutive low-promise readings within one attempt's verify sequence.
   It's checked per verify step (`crates/roko-cli/src/graph_task_dispatch/verification.rs`, "P4-03: PromiseTracker
   per-step check," ~line 493). Reported: verify runs fail fast, so a second reading never happens within the
   same attempt — `consecutive_low` can reach at most 1 before the verify sequence itself stops on the first
   failure, so B6's early-termination move never actually gets a chance to fire.
2. **B2 (`retry_delta`) is inert for ladder-routed tasks under default gates.** `retry_budget.rs`'s own test
   `the_ladder_raises_unauthored_budgets_to_its_floor` confirms `with_ladder_min_retries(5)` sets the ladder
   floor to 5; `GatesConfig::default().adaptive_max_retries` is also 5 by default (confirm the exact default at
   implementation time). With floor == adaptive_max, a `retry_delta` of `+1` has nowhere to go (already at the
   ceiling `adaptive_max_retries`) and `-1` gets clamped back up to the floor (`max_retries_with_delta`,
   `retry_budget.rs:159-161`, clamped within `[gates] adaptive_min_retries..=adaptive_max_retries`, and the
   ladder keeps raising unauthored budgets to its own floor regardless) — so M1's B2 move has no observable
   effect on a ladder-routed task's actual retry budget at these defaults.
3. **TierFloor moved down below θ₀ does nothing**, reportedly because backlog decision 8101's resolution logic
   takes `max(start, floor)` somewhere in the ladder's start-rung selection — a floor pushed *below* the
   baseline θ₀ start point is masked by that max, so the controller's "lower the floor" move has no effect once
   the floor is already below the baseline. (The exact call site applying `max(start, floor)` needs tracing at
   implementation time — `crates/roko-core/src/config/routing.rs::LadderConfig::resolve` computes `start` from a
   rung hint/tier default and doesn't show this max directly, so the floor-clamp likely happens in whichever
   caller passes `rung_hint` after consulting the controller's current `TierFloor` knob.)

## Why it matters

Goal: cybernetic, M1 controller (S06). Three of the controller's catalogued moves (B2, B6, and TierFloor-down)
look available in the move catalog but have no actual effect under realistic defaults/sequencing — the
controller can "choose" them, log a `param.change`, and nothing observable happens, which would show up as moves
that never produce a measurable `d_drive`, muddying any analysis of which moves actually help.

## Where

- `crates/roko-cli/src/runner/promise_tracker.rs::PromiseTracker`, `crates/roko-cli/src/graph_task_dispatch/verification.rs` (B6).
- `crates/roko-cli/src/graph_task_dispatch/retry_budget.rs` (B2, the ladder-floor/adaptive-max interaction).
- `crates/roko-learn/src/homeostasis/catalog.rs::KnobKind::TierFloor`, `crates/roko-core/src/config/routing.rs::LadderConfig::resolve` (TierFloor-down; exact clamp site needs tracing).

## Current state

All three confirmed inert under the described conditions; none fixed.

## Plan

1. For B6: either let the verify sequence continue past one failure when a `PromiseTracker` is active (so it can
   actually see a second reading), or redefine B6's trigger around what the current fail-fast sequencing can
   observe (e.g. one low reading plus some other signal).
2. For B2: either raise the default `adaptive_max_retries` above the ladder's default floor so `retry_delta` has
   room to move, or special-case ladder-routed tasks so B2 can still act (e.g. move the floor itself, not just
   the budget within a now-degenerate range).
3. For TierFloor-down: find the exact `max(start, floor)` call site (trace from `LadderConfig::resolve`'s
   callers) and either let a lowered floor actually lower the effective start, or document that TierFloor-down
   is a no-op below θ₀ by design, if that's the intended behavior.

## Done when

- Each of B6, B2 (ladder-routed) and TierFloor-down either has an observable effect under default settings, or
  is explicitly documented as intentionally inert in that regime.
- The `[[verify]]` command passes.

## Notes

- Discovered during PK63's work (gap-eb39c1, done). The exact 8101 clamp site for (3) needs more tracing than
  this pass did — flag that explicitly to whoever picks this up rather than guessing at a fix location.
