+++
id = "q-6b7cca"
kind = "question"
title = "Graph plan runs never emit PlanCompleted: wire the four plan-completion sinks (the dream sink starts claude) or remove them?"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
size = "S"
subsystem = ["roko-cli/graph_execution", "roko-cli/runtime_feedback"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "33e107da1"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:feat/learning-completion-loops 33e107da1"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan_body", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan", "crates/roko-cli/src/runtime_feedback/plan_completion.rs::DreamConsolidationSink", "crates/roko-cli/src/runtime_feedback/plan_completion.rs::DaimonPersistenceSink", "crates/roko-cli/src/runtime_feedback/plan_completion.rs::ThetaReflectionSink", "crates/roko-cli/src/runtime_feedback/plan_completion.rs::DeltaConsolidationSink", "crates/roko-core/src/config/learning.rs::LearningConfig"]
links = { depends_on = [], blocks = [], related = ["bug-470de8", "dec-e70592", "find-34a4b5"], supersedes = [], duplicate_of = "" }
+++

## Problem

`run_graph_plan_body` attaches four sinks to the Graph feedback facade (`plan_runner.rs:962-1008`):
`DreamConsolidationSink`, `DaimonPersistenceSink`, `ThetaReflectionSink` and
`DeltaConsolidationSink`. Each one reacts only to `FeedbackEvent::PlanCompleted`
(`runtime_feedback/plan_completion.rs:76`, `:202`, `:288`, `:410`), and no production code builds
that event. So on `roko plan run`, none of the four ever runs.

Emitting the event is a small change, but it would start a paid model call after every plan. Will
has to decide which sinks plan runs should keep.

## Why it matters

- **Dream.** The sink runs `DreamRunner::consolidate_async` with the agent command hard-coded to
  `"claude"` (`plan_completion.rs:116`), bounded at 300 s. Both gating flags default to true
  (`config/learning.rs:41`, `:214`), and `roko.toml:373` and `:379` set them. Emitting the event
  as things stand would therefore start a `claude` dream after every plan.
  - bug-470de8 (unverified) assumes this already happens today, and wants the default turned off.
  - dec-e70592 asks whether to park dreams and affect altogether.
- **Daimon.** Plan runs load `.roko/daimon/affect.json` (`plan_runner.rs:923`) and appraise it
  after every task (`graph_task_dispatch.rs:1996`); routing reads it. In roko-cli, only this sink
  writes it back (`plan_completion.rs:221`), so each plan run's affect updates are lost when the
  run ends.
- **Theta and delta.** Both only log. Their `CorticalState`, `ThetaConsumer` and `DeltaConsumer` are
  created fresh for each run (`plan_runner.rs:985-1008`) and never saved. Delta's three phases are
  stubs that produce diagnostic telemetry.

## Where

- `crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan_body`: builds the facade and
  the four sinks.
- `crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan` (`:1880`): where a plan ends.
  The event would go out once the outcome is known, next to the Graph TUI bridge's own
  PlanCompleted (`:2246`), carrying `succeeded`, the task counts and the settled plan spend.
- `crates/roko-cli/src/runtime_feedback/plan_completion.rs`: the four sinks.
- `crates/roko-core/src/config/learning.rs::LearningConfig`: the dream flags.

## Current state

Checked at 33e107da1: `grep -rn 'FeedbackEvent::PlanCompleted {' crates/roko-cli/src` finds only
two kinds of hit:
- match arms, for example `runner/conductor_adapter.rs:418`, which maps the event to a signal;
- tests: `plan_completion.rs:493` and later, `conductor_adapter.rs:970`, and the `tests/` e2e files.

Runner-v2's `event_loop.rs` emitted the event, and the emitter went with it in `6b5da8616`. The Graph
facade gained the sinks, but never an emitter.

## Plan

Options:

1. **Emit the event and keep all four sinks.**
   - Land bug-470de8 first, so a dream runs only when a config opts in.
   - This restores the documented behaviour, but costs one `claude` call per plan for anyone who
     opts in.
2. **Emit the event and keep only the daimon sink (recommended).**
   - Remove the dream, theta and delta sinks from the Graph facade. Dreams stay on demand, through
     `roko knowledge dream run`.
   - Affect carries across runs with no model cost, and nothing is spent on sinks that only log.
3. **Emit nothing and delete the four sinks from the Graph facade.**
   - Record that plan runs have no plan-completion hooks.
   - This fits dec-e70592's park option, but affect still resets on every run.

## Done when

- Will has picked an option. Record it here with the date.
- A follow-up gap is filed with a guarded test. For example: a plan run through `run_graph_plan`
  fires `PlanCompleted` exactly once per plan, and the chosen sinks see it.

## Notes

- `DreamConsolidationSink::on_event` returns straight after `tokio::spawn` (`plan_completion.rs:104`),
  so nothing waits for the dream, and a one-shot `roko plan run` can exit before it finishes. Under
  option 1, the dream must be awaited or bounded at shutdown.
- A multi-plan run should emit one event per plan, not one per run: the sinks key on `plan_id`.
