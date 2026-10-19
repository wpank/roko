+++
id = "bug-2110e8"
kind = "bug"
title = "Under a plan budget only a few turn caps wide, roko stops attempts as budget_exhausted at $0 spent: attempts that start beside one take the budget its settled calls freed, and its next call is refused instead of waiting"
status = "open"
triage = "verified"
severity = "p1"
goal = "core"
size = "M"
subsystem = ["roko-cli/budget"]
created = 2026-10-18
updated = 2026-10-18
last_verified = 2026-10-18
source = "gp2 golden-path rerun, rehearsal 2 (find-4003b9, gap-f30b8e; 2026-10-18)"
discovered_from = "gap-f30b8e"
anchors = ["crates/roko-cli/src/graph_task_dispatch/budget.rs::GraphPlanBudgetLedger", "crates/roko-cli/src/graph_task_dispatch/budget.rs::GraphPlanBudgetPolicy"]
links = { depends_on = [], blocks = [], related = ["bug-4fabd0", "gap-f30b8e", "find-4003b9", "gap-4665ac"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn an_attempt_in_flight_can_grow_after_waiters_take_the_freed_budget' crates/roko-cli/src && cargo test -p roko-cli --lib an_attempt_in_flight_can_grow_after_waiters_take_the_freed_budget"
+++

## Problem

Under a plan budget that fits only a few turn-cap holds, roko stops attempts as `budget_exhausted` with almost all of
the budget unspent. Seen at $0 on `15837120f` (work/find-4003b9, 2026-10-18), in rehearsal 2 of the golden-path
rerun (gap-f30b8e). The setup was the golden-path fixture, `max_plan_usd = 1.0`, the workspace's
`max_turn_usd = 0.5`, and a fake upstream that solves each task in three calls costing $0.00004 each.

- T2 and T4 start first and each reserve the turn cap, $0.50, so the ceiling is full. T1 and T3 wait: "a provider
  call waits for plan budget that calls in flight hold … spent_usd=0.0 reserved_usd=1.0 ceiling_usd=1.0".
- T2's and T4's first calls settle, and `record_call` trims each hold to what it spent plus one more call like the
  last. The waiting T1 and T3 proceed after 620 and 846 ms, each with a fresh hold of `min(turn cap, available)`,
  which is what was freed.
- T2's and T4's next calls must grow their holds by $0.00004, and nothing is unreserved. `admit_call` refuses at
  once: "plan budget exhausted: the attempt's next model call would cost about $0.0000, as much as its last, and
  only $0.0000 of the plan's $1.0000 cap is left". Both attempts settle `budget_exhausted` (not retried), so T6 and
  T9 never run and the plan fails, with $0.0008 spent of $1.00. T1 and T3, which took the freed budget, pass.

With `max_turn_usd = max_plan_usd / 8` (rehearsal 3, $0.95 and $0.119), the same run passes all nine tasks with no
wait and no refusal. The 2026-10-16 live runs had $4.00 and $0.50, room for eight holds, and never hit it.

## Why it matters

A small cap is the common case for a run nobody watches: bug-4fabd0's run came from the Hermes desktop page with a
$0.50 cap. With a `max_turn_usd` of $0.50, as roko's workspace `roko.toml` sets it, any plan with two tasks side by
side then fills its ceiling with one or two holds.
The second attempt's growth, or the first's, is refused, and the run fails as if it were over budget.
`budget_exhausted` is blamed on the harness and not retried, so nothing recovers. The golden-path acceptance rerun
under Will's $3 cap (about $0.95 a run) would have failed this way. It ran with a lowered `max_turn_usd` instead
(`work/history/golden-path-live-2026-10-18.md`).

## Where

- `crates/roko-cli/src/graph_task_dispatch/budget.rs::GraphPlanBudgetLedger::try_reserve`: a new attempt reserves
  `reservation_micro_usd()` (the turn cap) capped at `available`, so it can take every micro-dollar that settled
  calls just freed.
- `crates/roko-cli/src/graph_task_dispatch/budget.rs::GraphPlanBudgetLedger::admit_call`: an in-flight attempt
  whose next call needs `grow > available` is refused at once. It does not wait for other holds to shrink, though
  `reserve_waiting` does wait for a new attempt in the same position.
- `crates/roko-cli/src/graph_task_dispatch/budget.rs::GraphPlanBudgetPolicy::reservation_micro_usd`: the turn cap
  is the reservation whatever the ceiling's size. A `$0.50` cap under a `$1.00` ceiling leaves room for two attempts.

## Current state

Checked at `15837120f`: as above. Evidence (gitignored, scratch): the gp2 rehearsal 2 log `stderr.txt` lines 120-234
and its `result.json` in the session scratchpad's `gp2-run/gp/gp2-rehearsal-r2/evidence/`, copied to
`tmp/status-quo/2026-10-16-live/evidence/gp2/rehearsal/`.

## Plan

1. Let an in-flight attempt's growth come first. Either `admit_call` waits, as `reserve_waiting` does, while holds
   other attempts took after it started still sit above what they spent, or `try_reserve` leaves the in-flight
   holds room to grow (for example, never reserving past what keeps `MIN_ATTEMPT_CALLS` calls of growth for each
   attempt in flight).
2. Scale the reservation to the ceiling: reserve at most the ceiling's share per concurrent call when the turn cap
   is larger, so a $1 ceiling with a $0.50 turn cap does not admit two attempts and leave nothing over.
3. A unit test drives the rehearsal's sequence (two holds fill the ceiling, two attempts wait, the first calls
   settle, the waiters take the freed budget) and asserts that every attempt's next small call is admitted.

## Done when

- [ ] The `[[verify]]` test passes, and fails on `15837120f`.
- [ ] A $0 rerun of rehearsal 2 (fake solver, `max_plan_usd = 1.0`, `max_turn_usd = 0.5`) passes all nine tasks.

## Notes

- Found by the gp2 golden-path rerun (find-4003b9, gap-f30b8e). bug-4fabd0 added the per-attempt holds;
  bug-0bc2b4 and backlog 3102 set the reservation rules.
