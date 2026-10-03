+++
id = "gap-414e56"
kind = "gap"
title = "PK66 M3 self-model: After a low-confidence pass, request deeper verification from S05's ladder before… (+1 more)"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
rank = 66
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK66"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs"]
lane = "rust-cold"
parent = "spec-635697"
links = { depends_on = ["gap-cb5133", "gap-de0b87", "gap-0c429f", "gap-f0a7ee", "gap-940e44", "gap-2e4a81"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn low_confidence_pass_requests_depth_before_model' crates/roko-cli/src/ && cargo test -p roko-cli low_confidence_pass_requests_depth_before_model"

[[verify]]
command = "grep -rqw 'fn refine_spec_action_emits_event_and_keeps_the_ladder_default' crates/roko-cli/src/ && cargo test -p roko-cli refine_spec_action_emits_event_and_keeps_the_ladder_default"
+++

## Problem

This package delivers 2 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK66, slice 61xx, phase 8), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 6132 | M | p3 | After a low-confidence pass, request deeper verification from S05's ladder before escalating the model | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6132-after-a-low-confidence-pass-request-deeper-verification-from.md` |
| 2 | 6133 | S | p3 | Turn refine-spec and abandon forecasts into events and diagnoses, never silent changes | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6133-turn-refine-spec-and-abandon-forecasts-into-events-and.md` |

## Why it matters

Phase 8: M1 controller and guarded commit (S06). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6100-epic-m3-calibrated-self-model.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/graph_task_dispatch/self_model.rs`, `crates/roko-cli/src/graph_task_dispatch/verification.rs`.

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

- Waits on: PK17 (gap-cb5133), PK19 (gap-de0b87), PK52 (gap-0c429f), PK58 (gap-f0a7ee), PK60 (gap-940e44), PK64 (gap-2e4a81).
- Suggested model: opus.

## Progress

- 6132: implemented at f7c1cbcd1 on `work/gap-414e56`; cargo verification deferred to the batch gate.
  Policy (b)'s post-pass step (`self_model.rs`) hands d* to DP3, which applies max(its level, d*)
  (`verify_depth.rs::deepen_verification`); after the deepest depth, r > r_max rejects the pass and
  that failure climbs a rung. Active mode on self-started chains only; shadow logs; silent while
  p_fg is at its prior. d_j (prior 0.5) and c_j are defaults until S05 measures them.
- 6133: implemented at 72ad81814 on `work/gap-414e56`; cargo verification deferred to the batch gate.
  Both policies read S07's spec score; refine_spec/abandon publish `self_model_refine:`/
  `self_model_abandon:` diagnoses and event-log entries, and refine appends `spec.refine_requested`
  to the run's `spec.jsonl`. The attempt always runs on the ladder's choice; S07's gate does not
  read the request yet.
