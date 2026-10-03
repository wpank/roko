+++
id = "gap-26c055"
kind = "gap"
title = "Four minor M1/ladder bookkeeping gaps: a hardcoded exposure-count label, a static last-chance check, and two edge cases"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "M"
subsystem = ["roko-cli/graph-task-dispatch"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-9 follow-up reports 2026-10-03 (PK63 gap-eb39c1)"
discovered_from = "gap-eb39c1"
anchors = ["crates/roko-cli/src/graph_task_dispatch/decision_log.rs", "crates/roko-cli/src/graph_task_dispatch/ladder.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn last_chance_reads_the_live_retry_budget' crates/roko-cli/ && cargo test -p roko-cli last_chance_reads_the_live_retry_budget"
+++

## Problem

Four minor issues from PK63's work (gap-eb39c1, done), in the same M1/retry-ladder area:

1. **The `error_pattern_summary_top5` content-policy label ignores the live exposure count.**
   `crates/roko-cli/src/graph_task_dispatch/decision_log.rs:196`: `ContentDecisionPoint::ErrorPatterns =>
   "error_pattern_summary_top5"` — a hardcoded literal naming "top5" regardless of how many error patterns are
   actually exposed. Reported: a controller move ("B4") can change that count, so the label's number can disagree
   with reality. (Note: the label's *ranking description* being stale against PK33's work is already tracked
   separately as gap-a40021; this is the narrower "the number 5 is hardcoded" facet, not yet confirmed exactly
   which mechanism varies the count — flagged for the implementer to pin down, since the M1 catalog's own B-moves
   (`homeostasis/catalog.rs`) don't show a `B4` by that exact label in a direct search.)
2. **The ladder's last-chance check reads the task's static `spec.max_retries`, not its live retry budget.**
   `crates/roko-cli/src/graph_task_dispatch/ladder.rs:220`: `top && self.attempt_in_run(&task_key) >=
   spec.max_retries` — this compares against the task's *authored* spec value, not whatever M1's B2 `retry_delta`
   may have adjusted the task's live budget to (`retry_budget.rs`). A task whose live budget was raised above its
   authored spec could hit "last chance" before it's actually exhausted its real, adjusted budget.
3. **A floor raised mid-task can lift an already-climbed task past the configured cap.** If the controller
   raises a tier's floor after a task has already climbed partway up the ladder on its own, the new floor could
   push it to a rung above `adaptive_max_retries`'s intended ceiling for that task, since the floor-raise and the
   cap aren't reconciled against each other at the moment the floor changes.
4. **A θ swap between budget admission and the attempt's own row can split them (rare).** If the controller
   swaps the active θ (a param.change) in the narrow window between a task's budget being admitted under one θ
   and its attempt row being written, the two could end up reflecting different θ versions — a narrow race,
   reported as rare.

## Why it matters

Goal: cybernetic, M1 controller / retry ladder (S06). (1) is an observability/accuracy issue (a label lying about
its own count). (2) and (3) are both ways the ladder's retry-exhaustion bookkeeping can disagree with the
controller's own live adjustments, which could cut a task off early or let it climb further than intended. (4)
is a narrow correctness edge case in attribution.

## Where

- `crates/roko-cli/src/graph_task_dispatch/decision_log.rs:196` (1).
- `crates/roko-cli/src/graph_task_dispatch/ladder.rs:220` (2), and wherever the floor-raise/cap interaction (3)
  actually happens (needs tracing — likely `ladder.rs` or `retry_budget.rs`'s floor-setting path).
- The θ-swap/budget-admission/attempt-row sequencing (4) — needs tracing to the exact call sites at
  implementation time.

## Current state

(1) and (2) confirmed directly in code. (3) and (4) are as reported by PK63; not independently re-traced to an
exact line in this pass — flagged for the implementer to locate precisely.

## Plan

1. For (1): either make the label's count reflect the live exposure count, or rename it to not imply a fixed
   number if the count is meant to vary.
2. For (2): have the last-chance check compare against the task's live, delta-adjusted retry budget instead of
   the static spec value.
3. For (3) and (4): trace the exact mechanisms (floor-raise/cap reconciliation; θ-swap/admission/row-write
   ordering) before deciding a fix — both need more investigation than this pass covers.

## Done when

- The error-pattern label's count matches what's actually exposed (or the label no longer implies a fixed count).
- The last-chance check uses the task's live retry budget, not just its static spec value.
- (3) and (4) have been traced to their exact mechanisms and either fixed or explicitly accepted as rare/low-risk.
- The `[[verify]]` command passes (for at least 1 and 2; 3 and 4 may need their own verify once traced).

## Notes

- Discovered during PK63's work (gap-eb39c1, done). Related but distinct: gap-a40021 (the same label's ranking
  *description* being stale, a separate facet from this item's "hardcoded count" finding).
