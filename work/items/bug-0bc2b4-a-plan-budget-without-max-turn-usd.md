+++
id = "bug-0bc2b4"
kind = "bug"
title = "A plan budget without max_turn_usd fails concurrent tasks at reservation instead of queuing them"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "M"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:fix/graph-ready-queue 9ef6f4aad"
anchors = ["crates/roko-cli/src/graph_task_dispatch/budget.rs::GraphPlanBudgetPolicy::from_limits", "crates/roko-cli/src/graph_task_dispatch/budget.rs::GraphPlanBudgetLedger::reserve", "crates/roko-cli/src/graph_task_dispatch.rs::dispatch", "crates/roko-graph/src/engine.rs::execute_cell_with_retries"]
links = { depends_on = [], blocks = [], related = ["gap-e95077", "gap-4665ac", "gap-4d835d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn concurrent_tasks_wait_for_a_reserved_plan_budget' crates/roko-cli/src && cargo test -p roko-cli --lib concurrent_tasks_wait_for_a_reserved_plan_budget"
+++

## Problem

Set `[budget] max_plan_usd` above zero, leave `max_turn_usd` at its default of 0, and run a plan with
`max_parallel > 1`. The first task to dispatch reserves the whole remaining plan budget. Every task that starts
while that reservation is held fails at once with `BudgetExceeded { dimension: "plan_cost_micro_usd" }`, instead of
waiting until the reservation settles. The engine retries a failed cell immediately, with no backoff, so such a
task uses up its retry budget in microseconds and fails. Under the new skip-failed policy its dependants are then
skipped, and the plan reports failure with most of its budget unspent.

## Why it matters

A plan budget is the safety setting a user is most likely to turn on, and with it parallel plans fail for no
reason of their own. The ready-queue scheduler (445a60d0d) starts more tasks side by side, so this now happens
more often. roko's own `roko.toml` sets `max_turn_usd = 0.5`, which is why self-hosting runs have not shown it.

## Where

- `GraphPlanBudgetPolicy::from_limits` (`graph_task_dispatch.rs:352-370`): with no `max_turn_usd`, the per-call
  reservation is the whole ceiling. The comment there says this is so "only one unknown-cost call can be in flight at a time".
- `GraphPlanBudgetLedger::reserve` (:488-547): when spend plus reservations leave no capacity (`available == 0`),
  it returns `RokoError::BudgetExceeded`. It does not wait.
- `GraphTaskDispatcher::dispatch` (:3454-3456) and `dispatch_streaming` (:4477-4479) reserve first and return the
  error with `?`.
- `execute_cell_with_retries` (`crates/roko-graph/src/engine.rs:2785-2847`) retries a failed cell right away.
- `dispatch_stop` (:470-486) stops a plan only on settled spend. The test `only_settled_spend_at_the_ceiling_stops_dispatch`
  (:5513) shows that a whole-budget reservation "blocks further reservations but does not stop the plan", so the
  engine keeps starting tasks that then fail.
- The ceiling comes from `[budget] max_plan_usd` via `resolve_budget_ceiling` (`graph_execution/plan_runner.rs:510`).
  With `--budget X` the override path reserves nothing, so it is not affected.

## Current state

Checked at 33e107da1 by reading the code above. The parked, unverified gap-e95077 assumes that "calls run one at a
time" in this setup. In fact the extra calls fail. gap-4665ac (budget defaults) repeats that assumption.

## Plan

1. Make reservation wait for capacity: when only in-flight reservations block it (not settled spend), `reserve`
   waits, for example on a `tokio::sync::Notify` signalled by settle and release, and then tries again. It errors
   only once settled spend reaches the ceiling, which the dispatch stop already handles. The wait must also end on
   cancellation.
2. Optionally, as gap-e95077 suggests, reserve a bounded per-call estimate instead of the whole remainder when
   `max_turn_usd` is unset, so budgeted plans keep their parallelism.
3. Add `concurrent_tasks_wait_for_a_reserved_plan_budget` (roko-cli lib). With a ceiling, no `max_turn_usd` and two
   concurrent dispatches, the second waits and then runs once the first settles under the ceiling. Neither fails
   with `BudgetExceeded`.

## Done when

- With a plan budget and no `max_turn_usd`, concurrent tasks wait for budget capacity instead of failing, and a plan
  stops only on settled spend.
- The `[[verify]]` command passes.

## Notes
- 2026-10-01 (wk-scheduler): implemented on work/bug-28b604; cargo verification deferred to the batch check. This is
  Plan step 1. Step 2, a bounded per-call estimate, is left to gap-e95077.
  - `GraphPlanBudgetLedger::reserve_waiting`, which `dispatch` and `dispatch_streaming` now use, waits while only
    reservations in flight hold the plan's remaining budget. It wakes on a `tokio::sync::Notify` that `settle` and
    `release` signal, and rechecks every 250 ms.
  - It fails with `BudgetExceeded` once settled spend reaches the ceiling, which `dispatch_stop` also handles.
  - It ends with `Cancelled` once the cell context is cancelled. The non-waiting `reserve` is now test-only.
  - Tests: `concurrent_tasks_wait_for_a_reserved_plan_budget` (two concurrent dispatches, $1 plan, no
    `max_turn_usd`) and `a_waiting_reservation_ends_at_the_ceiling_or_a_stop`.
