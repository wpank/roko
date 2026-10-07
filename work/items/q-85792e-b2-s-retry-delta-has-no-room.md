+++
id = "q-85792e"
kind = "question"
title = "B2's retry_delta has no room to move on ladder-routed tasks: raise adaptive_max_retries, or amend 8101's clamp?"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-core/config"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-17b follow-up reports 2026-10-04 (bug-35a738, work/backlog-batch-17b)"
discovered_from = "bug-35a738 (open; confirms the exact defaults its own facet 2 left unconfirmed)"
anchors = ["crates/roko-core/src/config/gates.rs::GatesConfig", "crates/roko-cli/src/graph_task_dispatch/ladder.rs::LADDER_MIN_RETRIES"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

At the default gates, B2 (M1's `retry_delta` move) can't move on ladder-routed tasks, because
its entire adaptive range collapses onto the ladder's own floor. Confirmed precisely:
`GatesConfig::default()` sets `adaptive_min_retries = 3` and `adaptive_max_retries = 5`
(`crates/roko-core/src/config/gates.rs:92-98`). The ladder's own floor,
`LADDER_MIN_RETRIES = FAILURES_PER_RUNG * (MAX_ESCALATIONS + 1) - 1 = 2 * (2 + 1) - 1 = 5`
(`crates/roko-cli/src/graph_task_dispatch/ladder.rs:39-47`), is a hardcoded constant equal to
the adaptive ceiling, not derived from it. `TaskRetryBudgets::max_retries_with_delta`
(decision 8101, `crates/roko-cli/src/graph_task_dispatch/retry_budget.rs:159-174`) clamps any
`retry_delta` move to `[adaptive_min_retries, adaptive_max_retries] = [3, 5]`. So for an
unpinned, ladder-routed task (the common case once the ladder is on), its budget is already set
to the ladder's floor of 5 — the top of B2's own range — before M1 ever applies a delta: `+1`
has nowhere to go (already at the ceiling), and `-1` gets re-raised back to 5 by the ladder's
own floor-enforcement logic regardless. B2 can "choose" a move and log a `param.change`, but
nothing observable happens.

This is bug-35a738's own facet 2 (its original text already names this: "With floor ==
adaptive_max, a `retry_delta` of `+1` has nowhere to go... and `-1` gets clamped back up to the
floor") — this filing confirms the exact numbers (3, 5, 5) the original item's "confirm the
exact default at implementation time" left open.

It overlaps, but is distinct from, `gap-a4fbf8` (open): that item is about
`TaskRetryBudgets::for_task` discarding the adaptive gate-history suggestion's *source
attribution* (relabeling it `Ladder` even when the adaptive number was already correct or
higher) — a provenance/logging bug. This item is about B2's *numeric range* having no room to
move at all once the floor equals the ceiling. Fixing `gap-a4fbf8`'s attribution issue alone
would not give B2 any room to move; the two need separate decisions.

## Why it matters

Goal: cybernetic, M1 controller (S06), same goal as `bug-35a738`. A controller move that can be
selected, logged, and produces zero observable effect under realistic defaults muddies any
analysis of which moves actually help (exactly `bug-35a738`'s own framing) — and specifically
for B2, it means the controller's retry-budget lever is silently disabled for the majority
case (ladder-routed, unpinned tasks) without anyone deciding that on purpose.

## Where

- `crates/roko-core/src/config/gates.rs::default_min_retries`, `::default_max_retries` (3, 5).
- `crates/roko-cli/src/graph_task_dispatch/ladder.rs::LADDER_MIN_RETRIES`,
  `FAILURES_PER_RUNG`, `MAX_ESCALATIONS` (the ladder floor, also 5).
- `crates/roko-cli/src/graph_task_dispatch/retry_budget.rs::max_retries_with_delta` (decision
  8101's clamp).
- `gap-a4fbf8` (related, not a duplicate; see above).

## Plan (decision needed)

- **Option A — raise `adaptive_max_retries`'s default** above the ladder floor (e.g. to 6 or 7),
  so B2 has real room above the floor to move into, without changing the ladder's own
  floor-climbing logic.
- **Option B — amend decision 8101's clamp** for ladder-routed tasks specifically: let B2 move
  the ladder floor itself (not just the budget within an already-degenerate range), or exempt
  ladder-routed tasks from the adaptive ceiling entirely while they're climbing.
- Either way, decide whether B2 should also be able to move *below* the ladder floor for a
  ladder-routed task (currently impossible, since the ladder's own floor-raising logic wins),
  or whether that's intentionally out of scope.

## Done when

Will picks an option; B2 has an observable effect on a ladder-routed, unpinned task's retry
budget at the resulting defaults.

## Notes

- 2026-10-04 (wave-17b follow-up, bug-35a738, work/backlog-batch-17b not yet merged): confirmed
  at main HEAD `c796f09c1` (all three cited constants are unchanged by that branch's diff, so
  verifiable on `main` directly). Filed as `kind = "question"` per the instruction. A matching
  note has been added to `gap-a4fbf8` describing the overlap and the distinction.
