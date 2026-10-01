+++
id = "bug-2b1ddc"
kind = "bug"
title = "Interrupted Graph attempts lose their streamed usage when the plan runner abandons them after the 3 s drain"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "M"
subsystem = ["roko-cli/graph_execution", "roko-cli/graph-dispatch"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "e22421998"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:fix/dispatch-timeouts-cost e0673e3e0"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan", "crates/roko-cli/src/graph_execution/plan_runner.rs::INTERRUPT_DRAIN_TIMEOUT", "crates/roko-cli/src/graph_execution/plan_runner.rs::force_exit", "crates/roko-cli/src/graph_task_dispatch/budget.rs::GraphPlanBudgetReservation", "crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::dispatch"]
links = { depends_on = [], blocks = [], related = ["bug-690dc6", "q-1faa0c", "gap-36f3fb", "gap-b367bf", "bug-ceb581", "gap-288e38", "spec-b7303f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn an_interrupted_attempt_settles_the_usage_it_streamed' crates/roko-cli/src && cargo test -p roko-cli --lib an_interrupted_attempt_settles_the_usage_it_streamed"
+++

## Problem

Since `e0673e3e0` a Claude CLI attempt killed at its timeout reports the usage it streamed, marked estimated, and
Graph dispatch settles it. An attempt stopped by an interrupt (SIGINT/SIGTERM on `roko plan run`) can still end up
recorded as free. On the first signal the plan runner cancels the graph, SIGTERMs the in-flight agent trees, and waits
`INTERRUPT_DRAIN_TIMEOUT` (3 s) for the graph to settle. If it has not settled, the runner SIGKILLs the agents, sets
`flow_abandoned` and finalizes the checkpoint without awaiting the flow. The in-flight
`GraphTaskDispatcher::dispatch` future never returns, so its usage is never read:

- no `task_spend.record`;
- no `budget_reservation.settle`, and the reservation's `Drop` releases it with no spend;
- no `emit_feedback`, so there is no `costs.jsonl` row and no episode.

The process then exits and drops the future. A forced exit (a second signal, or `FORCED_EXIT_GRACE` 10 s after the
first) calls `std::process::exit` and loses the same data.

Expected: every attempt that ran settles its known spend (provider-reported or estimated) before the checkpoint is
finalized, including interrupted ones.

## Why it matters

Goal `core`. The attempts that get interrupted are usually the long, expensive ones. Their spend disappears from the plan
ledger in `costs.json`, from per-task spend, from `.roko/learn/costs.jsonl` and from learning. A resumed run therefore
starts with budget it has already spent. This is the cancellation remainder of `bug-690dc6` (timeouts, fixed in
`e0673e3e0`); `q-1faa0c` row 3 covers bounded settlement at shutdown.

## Where

- `crates/roko-cli/src/graph_execution/plan_runner.rs`:
  - `INTERRUPT_DRAIN_TIMEOUT` (:200) and `FORCED_EXIT_GRACE` (:203);
  - `force_exit` (:303);
  - `run_one_plan` (:1880): the watch loop cancels and SIGTERMs at :2160-2178; after the deadline it calls
    `kill_in_flight_agents` and `flow_abandoned = true` (:2147-2158), then `flow_result = None` (:2205) and the
    checkpoint is finished as interrupted.
- `crates/roko-cli/src/graph_task_dispatch.rs`:
  - the heartbeat `select!` loop around the provider call in `GraphTaskDispatcher::dispatch` (:4031-4055);
  - spend settlement only after it returns (:4078-4082);
  - `GraphPlanBudgetReservation`'s `Drop` (:620) releases an unsettled reservation.
- `crates/roko-agent/src/claude_cli_agent.rs` (:1107-1118): a Claude CLI run that dies from a signal already returns its
  streamed usage as an estimate. The dispatcher just never awaits it.

## Current state

Checked statically at `33e107da1`:

- An agent that exits within the 3 s drain is settled normally, because its dispatch future returns.
- An agent that takes longer loses its usage: for example, a Claude session whose tool subprocess ignores SIGTERM, or a
  stdout pipe that stays open.
- The TUI cancel (`control.cancel`) only calls `flow_handle.cancel()`. That lets running attempts finish, so it does not
  drop usage.
- Serve's plan cancel aborts the plan-run task after 500 ms (`roko-serve/src/routes/plans.rs:648-672`). Whether that
  loses usage too was not checked.

## Plan

1. After the drain deadline's SIGKILL, give the in-flight dispatches a short bounded window (for example 2 s, well under
   `FORCED_EXIT_GRACE`) to return, instead of breaking at once.
   - The killed Claude CLI run drains its output within `KILLED_OUTPUT_DRAIN_MS`.
   - The dispatcher's in-flight registry (`sibling_settle::InFlightTasks`) can tell the runner when every attempt has
     settled.
2. If a dispatch still has not returned, record what is known before the checkpoint is finalized: the reservation's
   attempt, the wall time, and any live usage the agent streamed. Mark it estimated rather than silently releasing it.
3. Await the `costs.jsonl` append for interrupted attempts. Today it is a fire-and-forget `tokio::spawn`, and the
   process exits right after finalizing.
4. Test `an_interrupted_attempt_settles_the_usage_it_streamed` (plan_runner tests, lib):
   - a fake `claude` prints one `assistant` event with usage, then traps SIGTERM and sleeps;
   - request an interrupt through `PlanRunInterruptHandle`;
   - assert that the plan's checkpoint cost ledger and `.roko/learn/costs.jsonl` both hold the streamed estimate.

## Done when

- An interrupted attempt's streamed spend appears in the plan ledger (`costs.json`), per-task spend and `costs.jsonl`,
  whether or not the agent obeyed SIGTERM within 3 s.
- The `[[verify]]` command passes.

## Notes

- `plan_runner.rs` and `graph_task_dispatch.rs` are hot files. Keep the change small and do not lengthen the forced-exit
  path: shutdown must stay bounded (the item `q-1faa0c` tests that).
- Implemented on `work/bug-2b1ddc` at `80aa04263` (formatting in `e22421998`); cargo verification deferred to the batch check. Premise re-checked at `faa378453`: still true. The fix follows bug-aa2044 rather than the plan's SIGKILL-and-wait: at the drain deadline the runner sets the plan's stop flag (`CellContext.cancel_flag`); `run_watched` drops a stopped attempt's call within 250 ms and the attempt settles through `AttemptProgress` with its streamed usage, estimated, as `AttemptOutcome::Cancelled`. That also covers API providers, which a SIGKILL never reaches. Stopped attempts get `INTERRUPT_SETTLE_TIMEOUT` (2 s) to settle; agents that ignored SIGTERM are SIGKILLed once they have, or once that window ends; an interrupted run waits up to 1 s for its row writes (`background_writes::settled`). Worst case about 6 s, under `FORCED_EXIT_GRACE`. `TaskExecutorCell` no longer retries a cancellation. With both stall thresholds 0 there is no `AttemptProgress`, so a stopped attempt settles with unknown usage. The verify test runs the plan in a child test process, because the interrupt signals every agent in its process.
