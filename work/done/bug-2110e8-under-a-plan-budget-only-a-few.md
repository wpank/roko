+++
id = "bug-2110e8"
kind = "bug"
title = "Under a plan budget only a few turn caps wide, roko stops attempts as budget_exhausted at $0 spent: attempts that start beside one take the budget its settled calls freed, and its next call is refused instead of waiting"
status = "done"
triage = "verified"
severity = "p1"
goal = "core"
size = "M"
subsystem = ["roko-cli/budget"]
created = 2026-10-18
updated = 2026-10-19
last_verified = 2026-10-19
last_verified_rev = "7d73f35e6"
source = "gp2 golden-path rerun, rehearsal 2 (find-4003b9, gap-f30b8e; 2026-10-18)"
discovered_from = "gap-f30b8e"
anchors = ["crates/roko-cli/src/graph_task_dispatch/budget.rs::GraphPlanBudgetLedger", "crates/roko-cli/src/graph_task_dispatch/budget.rs::GraphPlanBudgetPolicy"]
links = { depends_on = [], blocks = [], related = ["bug-4fabd0", "gap-f30b8e", "find-4003b9", "gap-4665ac"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn an_attempt_in_flight_can_grow_after_waiters_take_the_freed_budget' crates/roko-cli/src && cargo test -p roko-cli --lib an_attempt_in_flight_can_grow_after_waiters_take_the_freed_budget"

[closed]
at = 2026-10-19
at_ts = "2026-10-19T11:33:39Z"
commit = "7d73f35e6"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-19T10:30:36Z"
forced = false
evidence = "7d73f35e6 changes crates/roko-cli/src/graph_task_dispatch/budget.rs. When the unreserved budget cannot cover an in-flight attempt's next call, admit_call first takes the shortfall back from the holds admitted after it that have made no call (reclaimable_after and reclaim_after: newest first, each kept at attempt_floor), and only then refuses. Older holds keep what they hold. The [[verify]] test an_attempt_in_flight_can_grow_after_waiters_take_the_freed_budget uses rehearsal 2's settings: $1.00 plan, max_turn_usd $0.50, 8 agents, $0.00004 calls. It failed on 050f30ea8 (budget.rs as at 15837120f) at T2's call 3 with the rehearsal's own message, and the verify command passes verbatim at 7d73f35e6. a_call_takes_back_only_what_later_attempts_hold_beyond_their_floor covers the bounds. 475 graph_task_dispatch and budget lib tests pass (41 in the budget module at 7d73f35e6), and clippy -D warnings on roko-cli is clean. A $0 rerun of rehearsal 2 (fake solver, max_plan_usd 1.0, max_turn_usd 0.5, max_agents 8; bud-rehearsal in the session scratchpad): on gp2's 15837120f binary T3 settled budget_exhausted and 6 of 9 tasks ran. On 7d73f35e6 all nine passed first try; the plan was delivered and fast-forwarded, the 5 whole-plan steps passed, there were 0 budget refusals, and it cost $0.001194. T1's and T4's third calls came while T2 and T3, which took the budget they freed, had made no call. Plan step 2 (cap the reservation at the ceiling's share) was not done: it would shrink the Claude CLI's own --max-budget-usd below the turn cap. Filed gap-8bd8e2."
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

- [x] The `[[verify]]` test passes, and fails on `15837120f`. It fails on `050f30ea8`, whose `budget.rs` is the
  same as at `15837120f`, at T2's call 3 with the rehearsal's own message. It passes on `7d73f35e6`.
- [x] A $0 rerun of rehearsal 2 (fake solver, `max_plan_usd = 1.0`, `max_turn_usd = 0.5`) passes all nine tasks
  (2026-10-19, below).

## Notes

- Found by the gp2 golden-path rerun (find-4003b9, gap-f30b8e). bug-4fabd0 added the per-attempt holds;
  bug-0bc2b4 and backlog 3102 set the reservation rules.
- 2026-10-19 (claude-agent, `7d73f35e6`): fixed by plan step 1, as a take-back.
  - When the unreserved budget can't cover an in-flight attempt's next call, `admit_call` takes the shortfall back
    from the attempts that started after it and have made no call yet (`reclaim_after`). It takes from the newest
    first and leaves each `attempt_floor`. Attempts that started before it keep all they hold. Only past that is
    the call refused.
  - It is synchronous, so roko's tool loop and the CLI watch (gap-ca1c92) both get it through `refuse_call`.
- Plan step 2 (cap the reservation at the ceiling's share per agent) was not done. The Claude CLI's own
  `--max-budget-usd` (gap-e14344) is the hold plus the unreserved budget as its agent starts. In a busy $1 plan
  with 8 agents, a $0.125 share would stop a CLI attempt there, within its $0.50 turn cap. With the take-back, a
  turn cap that fills the ceiling costs only a wait for the first calls to settle (793 and 1030 ms in the rerun).
- The $0 rerun used `bud-rehearsal/rehearsal.py` in the session scratchpad, which is gp2's `gp --rehearsal`
  without the `max_plan_usd / 8` workaround. It had its own ports and fences, and its proxy forwards only to the
  local fake solver.
  - On gp2's `15837120f` binary: T3 settled `budget_exhausted` and 6 of 9 tasks ran.
  - On `7d73f35e6`: all nine passed first try, the plan was delivered and fast-forwarded, the five whole-plan
    steps passed, there were no refusals, and it cost $0.001194.
  - T1's and T4's third calls came while T2 and T3, which took the budget they freed, had not yet made a call.
- Found on the way: gap-8bd8e2. The reservation's routing budget reaches `RoutingInputs::budget_remaining_usd`, and
  nothing reads it.
