+++
id = "bug-050398"
kind = "bug"
title = "With affect on, every appraisal already autosaves affect.json, so the plan-end daimon save repeats it, and its unlocked write can put back an older state"
status = "open"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-cli/runtime_feedback", "roko-daimon"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "gap-a960a1 (2026-10-16)"
discovered_from = "agent:gap-a960a1 worker (work/gap-a960a1)"
anchors = ["crates/roko-cli/src/runtime_feedback/plan_completion.rs::DaimonPersistenceSink", "crates/roko-cli/src/graph_execution/plan_runner.rs::graph_daimon_state", "crates/roko-daimon/src/lib.rs::autosave", "crates/roko-cli/src/graph_task_dispatch/feedback.rs::emit_feedback"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-a960a1", "q-6b7cca", "dec-e70592", "spec-7a71d6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn daimon_plan_end_save_never_puts_back_an_older_state' crates/roko-cli/ && cargo test -p roko-cli daimon_plan_end_save_never_puts_back_an_older_state"
+++

## Problem

With `[daimon] enabled`, `.roko/daimon/affect.json` is already written after every labelled attempt, before any plan
completes:

- `graph_daimon_state` loads the run's affect state with `DaimonState::load_or_new`, which sets its
  `persistence_path`.
- Every `appraise` ends with `self.autosave()` (`roko-daimon/src/lib.rs`).
- W09 in `graph_task_dispatch/feedback.rs` appraises each labelled attempt while holding the state's lock.

So q-6b7cca's daimon premise was wrong: it said only the plan-completion sink writes affect back, so each run's
affect updates are lost. The plan-end save that gap-a960a1 wired (`DaimonPersistenceSink`) writes the same state
again.

That save also takes its snapshot under the lock but writes it outside the lock, through the same `affect.json.tmp`
that autosave uses. With `max_parallel_plans > 1`:

- plan A's completion can write a snapshot older than an autosave that a still-running plan B made in between. The
  file then holds the older state until B's next appraisal or B's completion;
- the two writers can collide on the shared tmp file. One rename fails, and the failure is only logged.

## Why it matters

Affect is held (dec-e70592) and off by default, so nothing breaks today. Once a workspace turns affect on and runs
plans side by side, the saved affect can briefly go backwards. And the save that q-6b7cca option 2 kept does not do
what its rationale said. Will decides whether the plan-end save stays: stage 1 of spec-7a71d6 kept it, and that epic's
done-when expects a `PlanCompleted`-driven daimon save.

## Where

- `crates/roko-cli/src/runtime_feedback/plan_completion.rs::DaimonPersistenceSink`: snapshot under the lock, then a
  `spawn_blocking` persist outside it.
- `crates/roko-cli/src/graph_execution/plan_runner.rs::graph_daimon_state`: `load_or_new`, which turns autosave on.
- `crates/roko-daimon/src/lib.rs::autosave`, and `persist`: always the same `affect.json.tmp`, then a rename.
- `crates/roko-cli/src/graph_task_dispatch/feedback.rs::emit_feedback`: W09, `appraise` under `daimon.lock()`.

## Current state

Found while doing gap-a960a1 (2026-10-16). To tell the plan-end save from autosave, its run test
(`graph_plan_run_emits_plan_completed_once_per_plan`) removes the affect file when an unlabelled attempt settles.

## Plan

Options:

1. Keep the plan-end save, but persist while holding the state's lock, as autosave does. Writes are then serialized
   and can never go backwards.
2. Drop the plan-end save as redundant and rely on autosave. This reopens q-6b7cca option 2, so Will decides.
3. Turn autosave off for plan runs (load without a persistence path) and keep the plan-end save as the one writer.
   Fewer writes, but a crash mid-plan loses that plan's affect.

Recommended: option 1. It changes nothing Will decided.

## Done when

The named test passes: it runs two plans side by side with affect on, and finds that `.roko/daimon/affect.json`
after the run equals the state that the last appraisal produced.

## Notes

- Affect stays held; this item does not turn it on.
