+++
id = "gap-2b5d37"
kind = "gap"
title = "PK11 Decision records and census: A frozen gate settlement writes no thresholds and makes no reflection call (+1 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
rank = 11
size = "M"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK11"
anchors = ["crates/roko-cli/src/graph_task_dispatch/gate_learning.rs", "crates/roko-cli/src/graph_task_dispatch/verification.rs"]
lane = "rust-hot"
parent = "spec-99d417"
links = { depends_on = ["gap-f61823"], blocks = [], related = ["gap-644040"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn frozen_gate_failure_writes_no_thresholds_or_reflections' crates/roko-cli/src/ && cargo test -p roko-cli --lib frozen_gate_failure_writes_no_thresholds_or_reflections"

[[verify]]
command = "grep -rqw 'fn frozen_learning_run_writes_no_learned_state' crates/roko-cli/src && cargo test -p roko-cli --lib frozen_learning_run_writes_no_learned_state"
+++

## Problem

This package delivers 2 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK11, slice 22xx, phase 2), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 2222 | S | p2 | A frozen gate settlement writes no thresholds and makes no reflection call | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2222-frozen-gate-settlement-writes-no-learned-state.md` |
| 2 | 2223 | M | p2 | A frozen fake-provider plan run leaves every learned-state file unchanged | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2223-frozen-plan-run-leaves-learned-state-unchanged.md` |

## Why it matters

Phase 2: honest measurement. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2200-decision-records-exposures-census-and-freeze.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/graph_execution/plan_runner.rs`, `crates/roko-cli/src/graph_task_dispatch/gate_learning.rs`, `crates/roko-cli/src/graph_task_dispatch/verification.rs`.

It also edits the hot file(s) `crates/roko-cli/src/graph_execution/plan_runner.rs`, which are left out of this item's anchors so that two hot packages can run at once; the coordinator resolves any merge conflict.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: PK10 (gap-f61823).
- Existing work items this package covers or touches: gap-644040. When its tasks are done, close those whose verify then passes.
- Suggested model: opus.

## Progress

- 2222: implemented at 035749c5c (frozen settle_gate_learning returns before the thresholds; the post-gate reflection is skipped). Cargo verification deferred to the batch gate.
- 2223: implemented at c243f761b (`frozen_learning_run_writes_no_learned_state` in plan_runner.rs; a frozen reflex check peeks instead of counting a hit; FeedbackService saves section effects and knowledge scores only when an outcome changed them). Cargo verification deferred to the batch gate. Its "Done when" also asks to close gap-644040 and tick it in spec-6ac537: left to the coordinator.
