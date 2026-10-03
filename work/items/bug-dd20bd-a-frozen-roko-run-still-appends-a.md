+++
id = "bug-dd20bd"
kind = "bug"
title = "A frozen roko run still appends a workflow_complete episode, because record_workflow_feedback ignores [learning] frozen"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/run"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-5 follow-up reports 2026-10-02 (PK11 gap-2b5d37)"
discovered_from = "gap-2b5d37"
anchors = ["crates/roko-cli/src/run.rs::record_workflow_feedback", "crates/roko-core/src/config/learning.rs::LearningConfig"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn frozen_roko_run_appends_no_episode_or_efficiency_row' crates/roko-cli/ && cargo test -p roko-cli frozen_roko_run_appends_no_episode_or_efficiency_row"
+++

## Problem

`roko run "<prompt>"` (the single-task Graph path, distinct from `roko plan run`) appends a `workflow_complete`
episode to the root `.roko/episodes.jsonl` even when `[learning] frozen = true`. `record_workflow_feedback`
(`crates/roko-cli/src/run.rs:926-955`) builds one `FeedbackEvent::GateResult` per gate plus a
`FeedbackEvent::WorkflowComplete`, and calls `feedback.record(event).await` for every one of them
unconditionally — no reference to `config.learning.frozen` anywhere in the function. Its one caller
(`run.rs:700`) is equally unconditional: `record_workflow_feedback(layout.root(), &report, outcome, duration).await;`
with no gate above it either.

## Why it matters

Decision 2218 (frozen runs produce no learning signal) is already implemented on the Graph *plan* dispatch path —
`crates/roko-cli/src/graph_task_dispatch/gate_learning.rs:265-267` has the established pattern: `if
self.learning_frozen() { /* records nothing */ }`, with its own test `frozen_gate_failure_writes_no_thresholds_or_reflections`.
`roko run`'s single-task path is a separate entry point that was never updated to match, so a frozen workspace
(used, per other wave-5 reports, for reproducible benchmark/demo runs where learning must not drift) still grows
`episodes.jsonl` by one `workflow_complete` row and appends to `learn/efficiency.jsonl` per gate, every time
`roko run` is used instead of `roko plan run`.

## Where

- `crates/roko-cli/src/run.rs::record_workflow_feedback` (lines 926-955): builds and unconditionally records the
  gate and completion feedback events.
- `crates/roko-cli/src/run.rs:700`: the one call site, also unconditional.
- Sibling, correct pattern: `crates/roko-cli/src/graph_task_dispatch/gate_learning.rs` (`learning_frozen()`,
  around line 267) — several other call sites across `graph_task_dispatch/feedback.rs`,
  `graph_task_dispatch/decision_log.rs`, `graph_task_dispatch/reflex_credit.rs` and `graph_execution/run_manifest.rs`
  check the same flag; `run.rs` is the odd one out.
- `crates/roko-core/src/config/learning.rs:193` (`pub frozen: bool`, default `false`).

## Current state

Unfixed. `roko run`'s config is available at the call site (`report`/`layout` are already in scope at `run.rs:700`;
the function would need the `RokoConfig` or just its `learning.frozen` bool threaded in, which is a small, local
change — it does not yet receive the config at all, per the current signature `(roko_dir: &Path, report: &WorkflowRunReport, outcome: String, duration: Duration)`).

## Plan

1. Thread `frozen: bool` (or the whole `&LearningConfig`) into `record_workflow_feedback`'s signature.
2. At the top of the function (or at the call site), return early without building or recording any
   `FeedbackEvent` when frozen — matching `gate_learning.rs`'s existing one-line guard style.
3. Add a test mirroring `frozen_gate_failure_writes_no_thresholds_or_reflections`'s shape, but for `roko run`'s
   path: a frozen config, a successful single-task run, and an assertion that `.roko/episodes.jsonl` and
   `learn/efficiency.jsonl` are unchanged (or absent) after it.

## Done when

- A frozen `roko run` appends no `workflow_complete` episode and no efficiency row.
- The `[[verify]]` command passes.

## Notes

- Keep the non-frozen behavior byte-identical; this is an early-return guard, not a rewrite of the feedback shape.
- `roko run` and `roko plan run` share no code at this call site today (confirmed: `record_workflow_feedback` is
  local to `run.rs`), so this fix is independent of anything on the Graph plan-dispatch path.
