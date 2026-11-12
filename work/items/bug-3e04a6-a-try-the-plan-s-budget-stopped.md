+++
id = "bug-3e04a6"
kind = "bug"
title = "A try the plan's budget stopped is never checked, so phase 6's T5 try 2, whose own check had passed 3 s before the stop, was thrown away and paid for again: at a budget stop roko should run the task's local checks on the work it left, with no model call, and pass the task when they pass"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "e1687649e"
source = "round v8 hermes-pilot-v8: phase 6 run 23837ede invocation 2 (Undertow 8945b3c, roko e722f0e1f), T5 try 2; the lead chose the fix"
discovered_from = "agent:hermes-pilot-v8 (round v8)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/try_allowance.rs::bounded_stop", "crates/roko-cli/src/graph_task_dispatch.rs", "crates/roko-cli/src/graph_task_dispatch/streaming.rs", "crates/roko-cli/src/graph_task_dispatch/helper_calls.rs::HelperCalls"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-2c9412", "bug-4fabd0", "gap-ca1c92"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_try_the_budget_stopped_passes_when_its_checks_pass' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_try_the_budget_stopped"
+++

## Problem

roko never checks a try that the plan's budget stopped, so it throws away a try whose own check has just passed.

Phase 6's T5 try 2 (run 23837ede, invocation 2, gpt-5.4):
- **What happened.** Its own `run_tests` of the task's check went from 8 of 9 cases to passed at 21:35:45 UTC. It
  updated its todo list at 21:35:47. At 21:35:48 roko stopped it, before "the attempt's next model call", which was
  the hand-off. No gate ran and no commit was made.
- **The work survived.** It stayed in the try's checkout (Console.h +76 −5 and a new Console.cpp of 646 lines,
  copied to `tmp/hermes-roko-v8/pilot/evidence/p6-T5-try2/`).
- **What it cost.** The try cost $1.205 and settled `budget_exhausted` with no steps. The run needed a third
  invocation and a person's approval. T5's try 3 then passed from that tree in 36 s, for $0.221.

`dispatch` (`graph_task_dispatch.rs`) settles a try with `TryStop::Plan` as `budget_exhausted` and returns before
`settle_task_verification`. So does the streaming path. gap-2c9412 made a try that ended at its rung's bound face its
checks, but its plan item 3 kept "a try the plan's budget stopped … is not verified".

## Why it matters

- The check is the source of truth: a try whose checks pass on the work it left has done its task.
- Throwing the try away costs a resume, a new try and a person's approval. Here, a third invocation of the run and
  $0.221 for T5's try 3.
- Goal: core.

## Where

- `crates/roko-cli/src/graph_task_dispatch/try_allowance.rs`: `bounded_stop` and `BoundedStop`, and
  `interrupted_at_bound` for an agent that runs its own loop.
- `crates/roko-cli/src/graph_task_dispatch.rs`: `dispatch`, which settles a try the budget stopped.
- `crates/roko-cli/src/graph_task_dispatch/streaming.rs`: the same on the streaming path.
- `crates/roko-cli/src/graph_task_dispatch/helper_calls.rs`: `HelperCalls`, the helper model calls of a verification.

## Plan

1. A try the plan's budget stopped ends at a bound, `BoundedStop::Budget`. The task's checks judge the work it left,
   as they judge a try that used up its rung's turns.
2. **No model money.** Its checks run in a local-only scope, so they make no model call:
   - `cheap_agent` and the judge's model give none, so there is no failure diagnosis, no LLM judge and no reflection;
   - the verify depth's model checks (B1, B3) are recorded as skipped.
3. **They pass:** the try passes, and its work is accepted as any passing try's is. Its verdict records the bound
   `budget`. The plan still starts no further task.
4. **They fail:** it settles `budget_exhausted` as before, with the steps it ran. The stop stays the harness's, never
   the agent's failure on its rung, and the plan stops.
5. Add the tests:
   - `a_try_the_budget_stopped_passes_when_its_checks_pass` (phase 6's T5): the try writes its fix, its own check
     passes, and the budget refuses the hand-off. The try passes, and no further model call is made;
   - `a_try_the_budget_stopped_whose_checks_fail_stays_budget_exhausted`: its checks run and fail, it settles
     budget_exhausted with their steps, and no model call diagnoses them.

## Done when

- The `[[verify]]` command passes.

## Notes

- Found by hermes-pilot-v8 (its findings, "Phase 6, the window"); the lead chose this fix ("At a budget stop, roko
  should run the task's local checks (no model money) on the stopped try's tree and pass the task if they're green").
- This changes gap-2c9412's plan item 3 for the plan's budget only. The cap keeps the last word on money, since the
  checks call no model. A try that ended because the run was stopping is still not checked.
- The audit lottery is unchanged. A green try draws its ticket as any does, and audits have their own budget,
  outside the plan's cap.
- Related: gap-2c9412, bug-4fabd0, gap-ca1c92.
