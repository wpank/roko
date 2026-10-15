+++
id = "gap-a960a1"
kind = "gap"
title = "Graph plan runs never emit PlanCompleted: emit it once per plan and keep only the daimon sink (q-6b7cca option 2)"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
size = "M"
subsystem = ["roko-cli/graph_execution", "roko-cli/runtime_feedback"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-20"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan", "crates/roko-cli/src/graph_execution/plan_runner.rs::build_graph_feedback_facade", "crates/roko-cli/src/graph_execution/plan_runner.rs::build_graph_feedback_context", "crates/roko-cli/src/runtime_feedback/plan_completion.rs::DreamConsolidationSink", "crates/roko-cli/src/runtime_feedback/plan_completion.rs::DaimonPersistenceSink", "crates/roko-cli/src/runtime_feedback/mod.rs::FeedbackEvent"]
lane = "rust-hot"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = ["q-6b7cca", "bug-470de8", "dec-e70592"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn graph_plan_run_emits_plan_completed_once_per_plan' crates/roko-cli/ && cargo test -p roko-cli graph_plan_run_emits_plan_completed_once_per_plan"

[[verify]]
command = "! grep -q 'DreamConsolidationSink::new' crates/roko-cli/src/graph_execution/plan_runner.rs && ! grep -q 'cognitive_clock_sinks()' crates/roko-cli/src/graph_execution/plan_runner.rs"
+++

## Problem

No production code builds `FeedbackEvent::PlanCompleted` on a Graph plan run. `grep -rn 'FeedbackEvent::PlanCompleted'
crates/roko-cli/src` at HEAD finds only match arms (`runner/conductor_adapter.rs:608`, `runtime_feedback/mod.rs:443`)
and tests (`runtime_feedback/plan_completion.rs:527` and later). So every sink that reacts only to that event is dead on
`roko plan run`. `build_graph_feedback_facade` still registers them (`graph_execution/plan_runner.rs:2673-2699`):

- `DreamConsolidationSink` (always registered; `dream_on_completion` now defaults to false after bug-470de8);
- `DaimonPersistenceSink` (registered when `[daimon] enabled` builds a daimon state; held, off by default);
- the theta and delta sinks, through `plan_completion::cognitive_clock_sinks()` under the parked `cognitive-clock`
  feature.

## Why it matters

CLAUDE.md priority 2 and NEW-FINDINGS finding 20. Will accepted q-6b7cca option 2 on 2026-10-15 (decisions 2 and 3,
`tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md`): emit the event once per plan, keep only the daimon sink, and
remove the dream, theta and delta sinks from the Graph facade. Dreams stay on demand (`roko knowledge dream run`).
Without the event, a workspace that turns affect on loses each run's affect updates when the run ends, because only
this sink writes `.roko/daimon/affect.json` back. Stage 1 of the epic.

## Where

- `crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan`: where a plan's outcome is known; the Graph TUI
  bridge closes the plan there (the `Graph TUI bridge: final status diff + PlanCompleted` block, ~:3810).
- `crates/roko-cli/src/graph_execution/plan_runner.rs::build_graph_feedback_facade`: registers the four sinks.
- `crates/roko-cli/src/graph_execution/plan_runner.rs::build_graph_feedback_context`: builds the facade, which then
  moves into the dispatcher (`.with_feedback(graph_feedback)`, ~:1489).
- `crates/roko-cli/src/runtime_feedback/plan_completion.rs`: the sinks; `DaimonPersistenceSink` stays.
- `crates/roko-cli/src/runtime_feedback/mod.rs::FeedbackEvent` (`PlanCompleted { plan_id, succeeded, tasks_completed,
  tasks_failed, total_cost_usd }`).

## Current state

Runner-v2's event loop emitted the event; the emitter went with it in `6b5da8616`. q-6b7cca (open since 2026-09-29)
describes the three options; option 2 is now chosen. Theta and delta are already off a default build (parked, 9222).

## Plan

1. Keep an `Arc` of the run's `FeedbackFacade` (clone `graph_feedback.feedback_facade` before it moves into the
   dispatcher) and give it to `run_one_plan` through `PlanRunContext`.
2. In `run_one_plan`, once the outcome is final (after the retry/fallback branch and before the TUI bridge closes the
   plan), send one `PlanCompleted` with the plan id, `succeeded`, the completed and failed task counts and the plan's
   settled spend. One event per plan in a multi-plan set, none for a plan that never started.
3. Remove `DreamConsolidationSink` and the `cognitive_clock_sinks()` loop from `build_graph_feedback_facade`. Delete
   the dream sink type if nothing else uses it (only `runtime_feedback/mod.rs:50` re-exports it); leave the theta and
   delta sinks to the parked feature or delete them, and say which in the commit.
4. Test `graph_plan_run_emits_plan_completed_once_per_plan`: a two-plan set through `run_graph_plan` with a fake
   provider and a counting sink sees exactly one event per plan with the right `succeeded` and counts; with
   `[daimon] enabled` the affect file is saved at plan end.

## Done when

Both `[[verify]]` commands pass: the named test passes, and `plan_runner.rs` registers neither the dream sink nor the
cognitive-clock sinks.

## Notes

- A frozen run (`[learning] frozen`) writes no learned state: decide whether the daimon save follows that flag and test
  it.
- Then update CLAUDE.md's priority 2 (it says Graph runs never emit `PlanCompleted`) and close q-6b7cca with this item
  as its follow-up.
- `DreamConsolidationSink::on_event` spawns and returns; with the sink gone there is nothing to await at exit.
