+++
id = "gap-de1e18"
kind = "gap"
title = "Split and replan never happen: ReplanController has no production caller, and the two-plan-failure circuit breaker is never fed"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
size = "L"
subsystem = ["roko-execution/replan", "roko-cli/graph_task_dispatch", "roko-conductor/circuit_breaker"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-21"
anchors = ["crates/roko-execution/src/replan_controller.rs::ReplanController", "crates/roko-cli/src/graph_task_dispatch/ladder.rs", "crates/roko-cli/src/graph_task_dispatch.rs", "crates/roko-conductor/src/circuit_breaker.rs::record_failure"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-ebd656"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn exhausted_ladder_replans_the_task_once' crates/roko-cli/ && cargo test -p roko-cli exhausted_ladder_replans_the_task_once"
+++

## Problem

Split and replan never happen. `ReplanController` (`crates/roko-execution/src/replan_controller.rs:295`; `decide`
:309, `apply` :417, with `ReplanRequest`, `ReplanStrategy`, `ReplanDecision`) has only its own tests as callers. A task
whose last attempt fails on its top rung "has exhausted the ladder: dispatch logs `ladder_exhausted` and tells the
dashboard to split or replan the task. Nothing splits it automatically" (`crates/roko-cli/src/graph_task_dispatch/
ladder.rs:10-16`). Every dispatch result sets `replan_strategy: None` and `split_into: None`
(`crates/roko-cli/src/graph_task_dispatch.rs:1975-1983`, `:2438-2446`).

Also unfed: the "two plan failures call a person" bound. `CircuitBreaker::record_failure`
(`crates/roko-conductor/src/circuit_breaker.rs:214`, `MAX_PLAN_FAILURES = 2`) has only test callers (D7 A.3).

## Why it matters

Goal `core`. NEW-FINDINGS finding 21; the papers name split/replan as a designed response to a disturbance (D3 C3.x2,
principle 6, Table 3.1; D5; D7 A.3), and the README pitch claims automatic split/replan (finding 33, another agent's).
Not on Will's turn-on list (decision 2), so lower priority and outside the cybernetic epic.

## Where

- `crates/roko-execution/src/replan_controller.rs::ReplanController` (`decide`, `decide_with_plan`, `apply`) and its
  receipt ledger (`to_extension`, checkpoint extension).
- `crates/roko-cli/src/graph_task_dispatch/ladder.rs` (`ladder_exhausted`), `crates/roko-cli/src/graph_task_dispatch.rs`
  (`replan_strategy`, `split_into` on the dispatch result).
- `crates/roko-conductor/src/circuit_breaker.rs::CircuitBreaker` (`record_failure`).
- The supervisor's `Nudge` and `ForceAdvance` (`graph_task_dispatch/supervision.rs:21`, :553) are gap-ebd656's (held).

## Current state

The controller is stateless and complete as a library (strategies, caps, receipts persisted in the checkpoint). Graph
has no step that turns a `ReplanDecision` into a plan mutation at run time.

## Plan

1. On `ladder_exhausted`, build a `ReplanRequest` from the task, its failures and prior receipts; call `decide_with_plan`.
2. For an accepted decision, apply the mutation to the running Graph (insert the split tasks or the rewritten task) with
   the cap the controller enforces, and record the receipt in the checkpoint so a resume does not replan twice.
3. Feed `CircuitBreaker::record_failure` once per failed plan run and stop (or ask a person) when it trips.
4. Test `exhausted_ladder_replans_the_task_once`: a fake-provider plan whose task fails on its top rung gets one replan
   receipt and a mutated task; a second exhaustion hits the cap.

## Done when

The `[[verify]]` command passes.

## Notes

- Lower priority: start after the cybernetic epic's stage 1. If Will prefers, close it as wontfix and change the papers
  and README instead (the papers stay as written for now, decision 2).
- A replan that rewrites a task changes what was authored; keep it behind a config switch (`[execution] replan`) that is
  off until Will says otherwise.
