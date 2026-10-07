+++
id = "q-3a1648"
kind = "question"
title = "Should holdout chains keep theta0's agent-slot count for A/B fairness?"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/homeostasis"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-13 follow-up reports 2026-10-04 (PK71 gap-099513, task 8134)"
discovered_from = "gap-099513 (closed; 8134 shipped a shared slot pool, fairness trade-off not decided)"
anchors = ["crates/roko-cli/src/graph_execution/agent_slots.rs::AgentSlotDispatcher", "crates/roko-learn/src/homeostasis/holdout.rs::params_for"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

The agent-slot pool is shared across every chain, so M1's live `max_parallel` (B5) also limits
holdout chains, not just the learned arm. `AgentSlotDispatcher` (8134,
`crates/roko-cli/src/graph_execution/agent_slots.rs:28-104`) is one `Semaphore` behind one
`theta: Option<HarnessParamsHandle>`: every slot request resizes the single shared pool to the
*current* controller's live `max_parallel` (`resize`, line ~96-104, "Size the pool by `theta`'s
`max_parallel` on each slot request"). The `TaskDispatcher` trait it implements carries no `Arm`
distinction, so a holdout-arm task and a learned-arm task draw from the exact same pool.
Meanwhile `params_for(Arm::Holdout, controller)` (`crates/roko-learn/src/homeostasis/holdout.rs:79-85`)
always returns `controller.state().theta0.clone()` — the *frozen baseline* params — specifically
so the holdout arm represents "what θ₀ would do," decoupled from wherever the live controller has
drifted to. But its effective concurrency is not decoupled: if the live controller's B5 shrinks
(e.g. responding to a disturbance), the shared pool shrinks too, and the holdout chain's actual
throughput shrinks with it, even though its params say θ₀.

## Why it matters

Goal: cybernetic, M1 controller / A-B holdout fairness (backlog 8134). The holdout arm exists to
give an unconfounded baseline to compare the learned arm against. If its concurrency silently
tracks the live controller's current state instead of staying pinned to θ₀, a difference in
outcome between the two arms can be partly a parallelism artifact (fewer/more concurrent agents
running at a given moment) rather than purely the controller policy's effect — undermining the
A/B comparison S06/M1 relies on.

## Where

- `crates/roko-cli/src/graph_execution/agent_slots.rs::AgentSlotDispatcher` (the one shared
  pool, resized by the live controller's θ).
- `crates/roko-learn/src/homeostasis/holdout.rs::params_for`, `HarnessHoldout` (the holdout arm,
  which assumes θ₀ governs its dispatch but doesn't govern its slot count).

## Current state

Confirmed by reading both files: `AgentSlotDispatcher` holds a single `Semaphore` and a single
`HarnessParamsHandle`, with no per-arm reservation or override; `params_for` returns θ₀ for the
holdout arm's dispatch params but has no path into `AgentSlotDispatcher` to pin its concurrency
separately.

## Plan (decision needed)

This is a design trade-off, not a straightforward bug fix — recorded as a question for a
decision:

- **Option A — keep the shared pool (status quo).** Simple, already shipped, one semaphore to
  reason about. The holdout arm's throughput reflects "what resourcing looks like under the
  live system's real operating conditions" rather than an idealized, isolated baseline. Risk:
  A/B comparisons confound controller-policy effects with parallelism effects whenever B5 moves.
- **Option B — give holdout chains a fixed reservation at θ₀'s `max_parallel`,** independent of
  the live controller's current B5 value (e.g. a second semaphore/reservation carved out of, or
  added to, `[conductor] max_agents`). True A/B fairness, at the cost of new resource-accounting
  complexity and a question of whether the two reservations should share or add to the
  `max_agents` cap.
- **Option C — hybrid:** keep one pool, but weight/attribute slot contention so a holdout
  chain's *measured* throughput is normalized against how many slots it actually got, rather
  than changing dispatch itself. Cheaper than B, but pushes the fairness correction into
  analysis rather than execution.

## Done when

Will picks an option (or another), and the chosen behavior is specified precisely enough to
anchor a follow-up `gap` item implementing it.

## Notes

- 2026-10-04 (wave-13 follow-up, PK71 8134): confirmed at main HEAD `b7ad508ce`. No `[[verify]]`
  command: this is a `kind = "question"` item pending Will's decision, not yet a coded fix.
