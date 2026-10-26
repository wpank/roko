+++
id = "gap-8bd8e2"
kind = "gap"
title = "A Graph attempt's plan-budget routing budget reaches RoutingInputs::budget_remaining_usd and nothing reads it, so model routing never sees what the plan has left"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/dispatch"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "bug-2110e8 worker, 2026-10-19 (budget.rs review on main 050f30ea8)"
discovered_from = "bug-2110e8"
anchors = ["crates/roko-cli/src/dispatch/model_routing.rs::RoutingInputs", "crates/roko-cli/src/graph_task_dispatch/budget.rs::effective_routing_budget"]
links = { depends_on = [], blocks = [], related = ["bug-4fabd0", "bug-2110e8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn routing_reads_what_the_plan_has_left' crates/roko-cli/src && cargo test -p roko-cli --lib routing_reads_what_the_plan_has_left"
+++

## Problem

A Graph attempt's plan-budget reservation carries a "routing budget" that is meant to steer model choice, and
nothing reads it. `GraphPlanBudgetReservation::routing_budget_usd` (what the attempt reserved) goes through
`effective_routing_budget` into `DispatchContext::budget_remaining_usd`. `RoutingInputs::from_task` then copies it
into `RoutingInputs::budget_remaining_usd`. No routing code reads that field: not `ModelRouter`, the ladder or
the cascade router. Model choice is the same with $0.01 left of the plan budget as with $100. Seen on `main`
`050f30ea8` while fixing bug-2110e8 (2026-10-19).

To see it: `grep -rn 'budget_remaining_usd' crates/roko-cli/src`. Every hit writes or copies the value, and
apart from test fixtures none reads it.

## Why it matters

bug-4fabd0's notes say "The reservation's routing budget only steers model choice", but it steers nothing. Either
the plan's remaining budget should reach routing (for example, a rung whose expected call cost exceeds what is left
is passed over), or the field and the plumbing should go, so nobody relies on a control that does not exist. It is
the "built but never connected" pattern.

## Where

- `crates/roko-cli/src/graph_task_dispatch/budget.rs::GraphPlanBudgetReservation::routing_budget_usd` and
  `effective_routing_budget`: the value's source.
- `crates/roko-cli/src/graph_task_dispatch.rs` (the `DispatchContext` built in `dispatch`) and
  `crates/roko-cli/src/graph_task_dispatch/streaming.rs`: where it is written.
- `crates/roko-cli/src/dispatch/model_routing.rs::RoutingInputs`: where it ends.

## Current state

Written in two places, copied once, read nowhere. Unrelated `budget_remaining` values elsewhere (roko-graph's
`CellContext::budget_remaining`, roko-execution's guards, roko-acp's session status) are separate.

## Plan

1. Decide: wire it (the router passes over a rung or model whose expected call cost the plan cannot pay), or
   remove `RoutingInputs::budget_remaining_usd`, `DispatchContext::budget_remaining_usd` and the reservation's
   routing budget.
2. If wired, a unit test (`routing_reads_what_the_plan_has_left`, the `[[verify]]`) shows a nearly spent plan
   routes differently from a fresh one. If removed, replace the `[[verify]]` with a check that the fields are gone.

## Done when

- The plan's remaining budget is read by a routing decision and a test shows it, or the dead fields are gone.
- The `[[verify]]` command passes.

## Notes

- Found by the bug-2110e8 worker (claude-agent, 2026-10-19). bug-2110e8 kept the reservation's size, so this is
  unchanged by it.
