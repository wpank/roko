+++
id = "gap-d1ebc1"
kind = "gap"
title = "M1 controller: no real DrivePredictor adapter, an inner/outer dead zone that idles forever, and a D_pre window that can straddle a disturbance's onset"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-learn/homeostasis"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-7 follow-up reports 2026-10-03 (PK62 gap-f7bab8)"
discovered_from = "gap-f7bab8"
anchors = ["crates/roko-learn/src/homeostasis/priors.rs::DrivePredictor", "crates/roko-learn/src/homeostasis/controller.rs::Controller"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn ev_between_inner_and_outer_bands_eventually_acts' crates/roko-learn/ && cargo test -p roko-learn ev_between_inner_and_outer_bands_eventually_acts"
+++

## Problem

Three design/implementation gaps in the M1 homeostat controller (S06), surfaced by PK62's work (gap-f7bab8, done):

1. **No adapter from the M3 SelfModel to the controller's `DrivePredictor`.** `crates/roko-learn/src/homeostasis/priors.rs::DrivePredictor` (trait, line 74) is the extension point: its own module doc comment says it "needs the dispatch-side task mix and the rung-to-model [map]" to predict the drive change the recent task mix causes. The only implementation in the codebase is a test `Stub` (`priors.rs:262`, used only by tests). Nothing wires the real M3 self-model's predictions into this trait for production use. Backlog task 8122 ("HomeostasisSink: one task resolution per chain") is related infrastructure (feeding task resolutions into the controller) but is not itself this adapter — confirm during implementation whether 8122 is a prerequisite or whether the predictor-side adapter needs its own task.
2. **S06's "nothing breached, so wait" can idle forever in the dead zone between an EV's inner and outer bands.** `Controller::idle` (`crates/roko-learn/src/homeostasis/controller.rs:974-995`) only acts when `self.latched` (confirmed outer-band breaches) is non-empty (opens an episode) or `self.relax_due` is true (nothing degraded, relax toward θ₀). An EV sitting between its inner and outer bands — degraded enough that it isn't healthy (so `relax_due` never fires) but not breached enough to latch — causes `idle()` to take neither action, indefinitely, every tick.
3. **D_pre's window can straddle a disturbance's onset, understating a step disturbance and causing a good first move to be rolled back.** `Controller::evaluate`'s `d_drive` (`controller.rs:451`, doc comment: "D over the dwell minus D over the window before the change") compares the dwell period's drive against a baseline computed over a fixed window immediately before the change. If a step disturbance's actual onset falls inside that pre-change window (rather than cleanly before it), the baseline average already includes some already-disturbed resolutions, inflating it — which shrinks the measured `d_drive` improvement a genuinely good first move produced, risking a correct move being judged ineffective and rolled back (and made tabu) when it actually helped.

## Why it matters

Goal: cybernetic, M1 controller (S06). (1) means the controller cannot yet act on M3's own calibrated
predictions in production — it's wired for a stub only. (2) and (3) are both ways the controller can behave
wrong on a real disturbance: (2) leaves a persistently-degraded EV unaddressed (no episode ever opens) as long as
it never crosses the outer-band threshold; (3) can cause thrashing (a correct fix rolled back, tabu'd, and a
worse move tried next) specifically around step disturbances, which is exactly the regime S06 cares most about
detecting and reacting to correctly.

## Where

- `crates/roko-learn/src/homeostasis/priors.rs::DrivePredictor`, `::Stub` (1).
- `crates/roko-learn/src/homeostasis/controller.rs::idle`, `::relax_due`, the `latched` set (2).
- `crates/roko-learn/src/homeostasis/controller.rs::evaluate`, `d_drive`'s window computation (3).
- `tmp/backlog/2026-10-02-complete-and-wire/8122-homeostasis-sink-one-resolution-per-chain.md` (related
  infrastructure for (1); confirm its relationship before assuming it covers the predictor-side adapter).

## Current state

All three confirmed in code as described; none addressed. (1) is a missing production implementation behind an
existing trait; (2) and (3) are both real gaps in `Controller`'s existing logic.

## Plan

1. Build the real `DrivePredictor` adapter over the M3 self-model, giving it the dispatch-side task mix and
   rung-to-model map its trait doc already specifies; confirm against 8122 whether that task supplies the inputs
   this adapter needs or whether a separate task is required.
2. Give the controller a policy for the inner/outer dead zone — e.g. a softer, lower-priority action (a smaller
   nudge, or a logged "degraded but not breached" signal) instead of pure inaction, so an EV can't sit degraded
   forever without being acted on.
3. Either widen `d_drive`'s pre-change window detection to exclude resolutions after a detected onset, or have
   the onset-detection logic (whatever flags the step disturbance in the first place) inform `evaluate`'s window
   boundary directly, so the baseline never averages in already-disturbed resolutions.

## Done when

- A real `DrivePredictor` implementation (not the test stub) is wired into the controller in production mode.
- An EV sitting between its inner and outer bands indefinitely triggers some controller action, not permanent
  inaction.
- A step disturbance whose onset lands inside the pre-change window no longer understates `d_drive` enough to
  roll back a genuinely good first move (verified with a synthetic onset-inside-window test case).
- The `[[verify]]` command passes.

## Notes

- Related: `gap-f7bab8` (done) itself, where a dated note now also records sub-finding (d) from this same
  wave-7 report — seed 7's S06 A1 claims both miss (first move is Ashby, not directed; detection lands at
  resolution 33, not 30) — not re-filed here since it's a refinement of an already-tracked, closed item's own
  flagged uncertainty, not a new mechanism.
