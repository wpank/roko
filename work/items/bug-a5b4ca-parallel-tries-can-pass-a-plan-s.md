+++
id = "bug-a5b4ca"
kind = "bug"
title = "Parallel tries can pass a plan's cap: each call is admitted against one more call like its attempt's last and booked at its real cost, so phase 6's three tries in flight stopped at $3.0048 of $3.00"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "c39fc94bc"
source = "round v8 hermes-pilot-v8 and run-analyst-v8: phase 6 run 23837ede (Undertow 8945b3c, roko e722f0e1f); filed by roko-planner-v8 at the lead's word"
discovered_from = "agent:hermes-pilot-v8 (round v8)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/budget.rs::admit_call", "crates/roko-cli/src/graph_task_dispatch/budget.rs::record_call"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-4fabd0", "bug-2110e8", "gap-ca1c92"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn parallel_calls_never_pass_the_plan_cap' crates/roko-cli/src/ && cargo test -p roko-cli --lib parallel_calls_never_pass_the_plan_cap"
+++

## Problem

A plan's spend can pass its cap when several attempts are in flight. Phase 6's first invocation (run 23837ede,
Undertow 8945b3c) stopped at $3.0048 of its $3.00 cap.

- **The tries.** T3 and T4 (gpt-5.4-mini) and T5 (gpt-5.4) ran in parallel through the OpenAI API, so each of their
  calls went through roko's tool loop and its budget check, `AttemptAllowance::refuse_call`, which calls
  `GraphPlanBudgetLedger::admit_call`.
- **The likely mechanism.**
  - `admit_call` lets a call go when the plan can cover what the attempt has spent plus one more call like its last.
  - `record_call` then books what the call really cost, even when that was more than it held.
  - Three calls in flight that each cost more than their last can pass the cap together.
- **The stop came after the overrun.** roko's stop line, "the attempt's next model call would cost about $0.0090, as
  much as its last, and only $0.0023 of the plan's $3.0000 cap is left", came after the overrun was booked. T3, T4
  and T5 settled as `budget_exhausted` within 35 s of each other, at $0.4104, $0.4453 and $0.7654.
- **Not yet confirmed.** The run's records keep each attempt's total, not each call's, so the costs of the last calls
  are not on record. The test below rebuilds the case.

## Why it matters

- A cap a person approved is a promise. $0.0048 is small, but the gap grows with the number of tries in flight and
  the size of a call: a call that writes a whole file can cost several times the one before it.
- Goal: core.

## Where

- `crates/roko-cli/src/graph_task_dispatch/budget.rs`:
  - `PlanBudgetState::admit_call`, which holds room for the next call;
  - `GraphPlanBudgetLedger::record_call`, which books its real cost;
  - `AttemptAllowance`, the tool loop's check before each call.

## Plan

1. Bound an admitted call by what its hold can pay. Either give the provider call a maximum output that the hold can
   pay at the model's output price, or reserve the plan's costliest call so far (`costliest_call_micro_usd`) rather
   than the attempt's last while several attempts are in flight.
2. Add the test `parallel_calls_never_pass_the_plan_cap`. Three attempts are in flight on a $3.00 cap, each call
   costing more than its last. The plan's spend stays within the cap, and the attempts that cannot go on stop as
   budget_exhausted.

## Done when

- The `[[verify]]` command passes.

## Notes

- Evidence: hermes-pilot-v8's findings ("Phase 6, the window": "Overrun of $0.0048 (0.16%)"); the run's
  `attempts.jsonl` in Undertow's `.roko/runs/23837ede-1d23-4d53-b8a9-fbfa362760d0/` (read-only).
- Related: bug-4fabd0 (holds inside an attempt), bug-2110e8 (an attempt in flight comes first), gap-ca1c92 (CLI
  agents, which are told the cost as it streams).
